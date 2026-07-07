use std::fs;
use std::path::Path;

use super::{Check, CheckResult};

const TODO_PATTERNS: &[&str] = &["todo!()", "unimplemented!()"];

fn is_test_file(path: &Path) -> bool {
    let path_str = path.to_string_lossy().replace('\\', "/");
    if path_str.contains("/tests/") || path_str.contains("\\tests\\") {
        return true;
    }
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    stem.ends_with("_test") || stem == "tests"
}

fn strip_string_literals(line: &str) -> String {
    let mut result = String::new();
    let mut in_string = false;
    let mut in_char = false;
    let mut prev_backslash = false;
    for ch in line.chars() {
        if prev_backslash {
            prev_backslash = false;
            continue;
        }
        if ch == '\\' && (in_string || in_char) {
            prev_backslash = true;
            continue;
        }
        if ch == '"' && !in_char {
            in_string = !in_string;
            continue;
        }
        if ch == '\'' && !in_string {
            in_char = !in_char;
            continue;
        }
        if in_string || in_char {
            continue;
        }
        result.push(ch);
    }
    result
}

fn scan_file(path: &Path) -> Vec<String> {
    let mut hits = Vec::new();
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return hits,
    };

    let mut in_test_mod = false;
    for (line_no, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.contains("#[cfg(test)]") {
            in_test_mod = true;
        }
        if in_test_mod
            && (trimmed.starts_with("mod ") || trimmed.starts_with("pub mod "))
            && trimmed.ends_with('{')
            && !trimmed.contains("test")
        {
            in_test_mod = false;
        }
        if in_test_mod || trimmed.contains("#[test]") {
            continue;
        }

        let code = strip_string_literals(trimmed);
        let code_no_comment = code.split("//").next().unwrap_or("").trim();

        for pattern in TODO_PATTERNS {
            if code_no_comment.contains(pattern) {
                hits.push(format!("{}:{}: {}", path.display(), line_no + 1, trimmed.trim()));
                break;
            }
        }
    }
    hits
}

pub struct TodoScanCheck;

impl Check for TodoScanCheck {
    fn name(&self) -> &'static str {
        "TODO/FIXME scan"
    }

    fn blocking(&self) -> bool {
        true
    }

    fn run(&self) -> CheckResult {
        let mut all_hits = Vec::new();
        let crates_dir = Path::new("crates");
        if !crates_dir.exists() {
            return CheckResult {
                name: self.name(),
                passed: true,
                blocking: true,
                message: "no crates/ directory found".into(),
            };
        }

        if let Ok(entries) = walk_rs(crates_dir) {
            for path in entries {
                if !is_test_file(&path) {
                    all_hits.extend(scan_file(&path));
                }
            }
        }

        if all_hits.is_empty() {
            CheckResult {
                name: self.name(),
                passed: true,
                blocking: true,
                message: "no todo!/unimplemented! found in non-test code".into(),
            }
        } else {
            let count = all_hits.len();
            CheckResult {
                name: self.name(),
                passed: false,
                blocking: true,
                message: format!(
                    "{} placeholder(s) found:\n    {}",
                    count,
                    all_hits.into_iter().take(10).collect::<Vec<_>>().join("\n    ")
                ),
            }
        }
    }
}

fn walk_rs(dir: &Path) -> Result<Vec<std::path::PathBuf>, std::io::Error> {
    let mut results = Vec::new();
    fn walk_inner(dir: &Path, results: &mut Vec<std::path::PathBuf>) -> Result<(), std::io::Error> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                walk_inner(&path, results)?;
            } else if path.extension().map(|e| e == "rs").unwrap_or(false) {
                results.push(path);
            }
        }
        Ok(())
    }
    walk_inner(dir, &mut results)?;
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_test_file() {
        assert!(is_test_file(Path::new("crates/foo/tests/test_bar.rs")));
        assert!(!is_test_file(Path::new("crates/foo/src/lib.rs")));
    }

    #[test]
    fn test_strip_string_literals() {
        assert_eq!(strip_string_literals(r#"let s = "todo!()";"#), "let s = ;");
        assert_eq!(
            strip_string_literals(r#"const X: &str = "unimplemented!()";"#),
            "const X: &str = ;"
        );
        assert_eq!(strip_string_literals("todo!();"), "todo!();");
    }

    #[test]
    fn test_actual_todo_detected() {
        let dir = std::env::temp_dir().join("axiom-test-todo");
        let _ = fs::create_dir_all(&dir);
        let file = dir.join("sample.rs");
        fs::write(&file, "fn x() { todo!(); }\n").unwrap();
        let hits = scan_file(&file);
        assert_eq!(hits.len(), 1);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_string_literal_todo_not_detected() {
        let dir = std::env::temp_dir().join("axiom-test-todo2");
        let _ = fs::create_dir_all(&dir);
        let file = dir.join("sample.rs");
        fs::write(&file, r#"const MSG: &str = "todo!() placeholder";"#).unwrap();
        let hits = scan_file(&file);
        assert!(hits.is_empty(), "string literal todo should not be flagged: {:?}", hits);
        let _ = fs::remove_dir_all(&dir);
    }
}

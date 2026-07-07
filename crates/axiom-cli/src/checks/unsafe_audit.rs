use std::fs;
use std::path::Path;

use super::{Check, CheckResult};

fn strip_string_literals_and_comments(line: &str) -> String {
    let mut result = String::new();
    let mut in_string = false;
    let mut in_char = false;
    let mut in_line_comment = false;
    let mut prev_backslash = false;
    for ch in line.chars() {
        if in_line_comment {
            break;
        }
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
        if ch == '/' && result.ends_with('/') {
            result.pop();
            in_line_comment = true;
            continue;
        }
        result.push(ch);
    }
    result
}

fn contains_actual_unsafe(code: &str) -> bool {
    let trimmed = code.trim();
    if trimmed.starts_with("unsafe fn")
        || trimmed.starts_with("unsafe trait")
        || trimmed.starts_with("unsafe impl")
        || trimmed.starts_with("unsafe extern")
    {
        return true;
    }
    let words: Vec<&str> = trimmed.split_whitespace().collect();
    for (i, w) in words.iter().enumerate() {
        if w == &"unsafe" {
            let next = words.get(i + 1).copied().unwrap_or("");
            if next.starts_with('{')
                || next == "{"
                || next == "fn"
                || next == "trait"
                || next == "impl"
                || next == "mod"
                || next == "extern"
            {
                return true;
            }
        }
    }
    false
}

fn line_has_safety_comment(lines: &[&str], line_idx: usize) -> bool {
    for back in 1..=3 {
        if let Some(prev_idx) = line_idx.checked_sub(back) {
            let prev = lines[prev_idx].trim();
            if prev.contains("SAFETY:") {
                return true;
            }
        }
    }
    false
}

fn scan_file(path: &Path) -> Vec<String> {
    let mut hits = Vec::new();
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return hits,
    };

    let lines: Vec<&str> = content.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        let code = strip_string_literals_and_comments(line);
        if contains_actual_unsafe(&code) && !line_has_safety_comment(&lines, i) {
            hits.push(format!(
                "{}:{}: {} (missing // SAFETY: comment)",
                path.display(),
                i + 1,
                line.trim()
            ));
        }
    }
    hits
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

pub struct UnsafeAuditCheck;

impl Check for UnsafeAuditCheck {
    fn name(&self) -> &'static str {
        "unsafe code audit"
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
                all_hits.extend(scan_file(&path));
            }
        }

        if all_hits.is_empty() {
            CheckResult {
                name: self.name(),
                passed: true,
                blocking: true,
                message: "no unsafe code without SAFETY comment".into(),
            }
        } else {
            let count = all_hits.len();
            CheckResult {
                name: self.name(),
                passed: false,
                blocking: true,
                message: format!(
                    "{} unsafe violation(s):\n    {}",
                    count,
                    all_hits.into_iter().take(10).collect::<Vec<_>>().join("\n    ")
                ),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detects_unsafe_block() {
        assert!(contains_actual_unsafe("unsafe {"));
        assert!(contains_actual_unsafe("unsafe { x }"));
        assert!(contains_actual_unsafe("unsafe fn foo()"));
        assert!(contains_actual_unsafe("unsafe impl Send for Foo"));
        assert!(contains_actual_unsafe("unsafe trait Foo"));
    }

    #[test]
    fn test_ignores_unsafe_in_strings() {
        assert!(!contains_actual_unsafe("let msg = \"unsafe {\""));
        assert!(!contains_actual_unsafe("println!(\"unsafe block\")"));
    }

    #[test]
    fn test_strip_comments() {
        let code = strip_string_literals_and_comments("let x = 1; // unsafe {");
        assert!(!contains_actual_unsafe(&code));
    }
}

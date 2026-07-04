use crate::checker::Violation;

pub fn report_text(violations: &[Violation]) -> String {
    if violations.is_empty() {
        return "No architecture violations found.".to_string();
    }

    let mut out = String::new();
    out.push_str(&format!(
        "Architecture violations: {}\n\n",
        violations.len()
    ));

    for (i, v) in violations.iter().enumerate() {
        out.push_str(&format!("{}. [{}] {}\n", i + 1, v.severity, v.category));
        out.push_str(&format!("   {}\n", v.message));
        if let Some(file) = &v.file {
            out.push_str(&format!("   File: {}\n", file.display()));
        }
        if let Some(line) = v.line {
            out.push_str(&format!("   Line: {}\n", line));
        }
        out.push('\n');
    }

    out
}

pub fn report_json(violations: &[Violation]) -> String {
    let report = serde_json::json!({
        "violations": violations.iter().map(|v| {
            serde_json::json!({
                "severity": match v.severity {
                    crate::checker::Severity::Blocker => "BLOCKER",
                    crate::checker::Severity::Warning => "WARNING",
                },
                "category": v.category,
                "message": v.message,
                "file": v.file.as_ref().map(|p| p.to_string_lossy().to_string()),
                "line": v.line,
            })
        }).collect::<Vec<_>>(),
        "summary": {
            "total": violations.len(),
            "blockers": violations.iter().filter(|v| v.severity == crate::checker::Severity::Blocker).count(),
            "warnings": violations.iter().filter(|v| v.severity == crate::checker::Severity::Warning).count(),
        }
    });

    serde_json::to_string_pretty(&report).expect("JSON serialization of architecture report failed")
}

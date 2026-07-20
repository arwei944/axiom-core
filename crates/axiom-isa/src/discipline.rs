//! T8 — Atom / Port discipline (constitution L4 / L8).
//!
//! Business side effects **must** go through [`crate::Port`] and be journaled via
//! [`crate::run_port`]. Pure logic uses [`crate::Atom`] + [`crate::run_atom`].
//! Shape conversion uses [`crate::Adapter`] + [`crate::run_adapter`].
//!
//! This module provides:
//! 1. Policy constants (banned I/O tokens outside Port impls).
//! 2. Source-text scanner for commercial crates (CI / path tests).
//! 3. **Auto-discovery** of Composer-bearing sources under commercial crate trees.
//! 4. Runtime journal-prefix helpers for step-kind verification.

use crate::error::{IsaError, IsaResult};
use crate::primitives::StepKind;
use std::path::Path;

/// Baseline commercial sources (always scanned). Discovery may add more.
pub const COMMERCIAL_ISA_SOURCES: &[&str] = &[
    "crates/axiom-demo-taskflow/src/pipeline.rs",
    "crates/axiom-demo-taskflow/src/workbench.rs",
    "crates/axiom-demo-taskflow/src/task_cell.rs",
    "crates/axiom-demo-taskflow/src/agent_cell.rs",
    "crates/axiom-demo-taskflow/src/llm_port.rs",
];

/// Crate source roots walked by [`discover_composer_sources`].
pub const COMMERCIAL_CRATE_SRC_ROOTS: &[&str] = &["crates/axiom-demo-taskflow/src"];

/// Tokens that indicate raw I/O and must not appear in Atom-only helpers.
/// Port `impl` blocks may contain I/O; the scanner exempts lines inside
/// `impl Port` … matching braces (best-effort line heuristics).
pub const BANNED_SIDE_EFFECT_TOKENS: &[&str] = &[
    "std::fs::",
    "tokio::fs::",
    "std::net::TcpStream",
    "reqwest::",
    "ureq::",
    "sqlx::",
    "rusqlite::",
    "Command::new",
    "std::process::",
];

/// Required ISA entry helpers in commercial composers.
pub const REQUIRED_ISA_HELPERS: &[&str] = &["run_atom", "run_port"];

/// Scan source text for discipline violations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisciplineViolation {
    pub line: usize,
    pub kind: String,
    pub detail: String,
}

/// Result of scanning one source file.
#[derive(Debug, Clone, Default)]
pub struct DisciplineReport {
    pub path: String,
    pub violations: Vec<DisciplineViolation>,
    pub saw_run_atom: bool,
    pub saw_run_port: bool,
    pub saw_atom_trait_or_fn: bool,
    pub saw_port_trait_or_fn: bool,
}

/// True when source text looks like a Composer orchestration module (not a pure cell shell).
pub fn is_composer_bearing_source(path: &str, source: &str) -> bool {
    let p = path.replace('\\', "/");
    // Cell shells hold a Composer but do not implement ISA steps themselves.
    if p.ends_with("_cell.rs") || p.contains("/cells/") {
        return false;
    }
    // Port-only modules (e.g. llm_port.rs) — banned I/O scan only.
    if p.ends_with("llm_port.rs") || p.contains("/ports/") {
        return false;
    }
    if p.contains("pipeline") || p.contains("workbench") {
        return true;
    }
    // Construction / impl sites — not mere type mentions in fields.
    if source.contains("SeqComposer::new") {
        return true;
    }
    if source.contains("impl Composer")
        || (source.contains("impl<") && source.contains(" for ") && source.contains("Composer"))
    {
        return true;
    }
    // Explicit orchestration helpers used together.
    source.contains("run_atom")
        && source.contains("run_port")
        && (source.contains("WitnessJournal") || source.contains("fn compose"))
}

/// Discover Composer-bearing `.rs` files under commercial crate trees (relative paths).
///
/// Unions baseline [`COMMERCIAL_ISA_SOURCES`] with auto-discovered paths so new
/// Composer modules cannot evade CI by omitting a manual list entry.
pub fn discover_composer_sources(workspace: impl AsRef<Path>) -> Vec<String> {
    let root = workspace.as_ref();
    let mut found: Vec<String> = COMMERCIAL_ISA_SOURCES
        .iter()
        .map(|s| (*s).to_string())
        .collect();

    for rel_root in COMMERCIAL_CRATE_SRC_ROOTS {
        let dir = root.join(rel_root);
        if !dir.is_dir() {
            continue;
        }
        walk_rs(&dir, root, &mut found);
    }

    found.sort();
    found.dedup();
    found
}

fn walk_rs(dir: &Path, workspace: &Path, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for ent in entries.flatten() {
        let path = ent.path();
        if path.is_dir() {
            walk_rs(&path, workspace, out);
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        // Skip pure module roots without orchestration markers unless baseline.
        let Ok(src) = std::fs::read_to_string(&path) else {
            continue;
        };
        let rel = path
            .strip_prefix(workspace)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        if is_composer_bearing_source(&rel, &src) || out.iter().any(|x| x == &rel) {
            if !out.iter().any(|x| x == &rel) {
                out.push(rel);
            }
        }
    }
}

/// Scan all discovered commercial sources; returns failing reports only.
pub fn scan_workspace_commercial(workspace: impl AsRef<Path>) -> Vec<DisciplineReport> {
    let root = workspace.as_ref();
    let mut bad = Vec::new();
    for rel in discover_composer_sources(root) {
        let path = root.join(&rel);
        let Ok(src) = std::fs::read_to_string(&path) else {
            bad.push(DisciplineReport {
                path: rel,
                violations: vec![DisciplineViolation {
                    line: 0,
                    kind: "missing-file".into(),
                    detail: "discovered path unreadable".into(),
                }],
                ..Default::default()
            });
            continue;
        };
        let report = scan_source(&rel, &src);
        if !report.ok() {
            bad.push(report);
        }
    }
    bad
}

impl DisciplineReport {
    /// Composer sources must call `run_atom`/`run_port`.
    /// Cell hosts only need zero banned side-effects (they compose via workbench/pipeline).
    pub fn requires_isa_helpers(&self) -> bool {
        // Path-only fallback when source not retained on report.
        self.path.contains("pipeline")
            || self.path.contains("workbench")
            || (self.saw_run_atom && self.saw_run_port && !self.path.contains("llm_port"))
    }

    pub fn ok(&self) -> bool {
        if !self.violations.is_empty() {
            return false;
        }
        if self.requires_isa_helpers() {
            self.saw_run_atom
                && self.saw_run_port
                && self.saw_atom_trait_or_fn
                && self.saw_port_trait_or_fn
        } else {
            true
        }
    }

    pub fn summary(&self) -> String {
        if self.ok() {
            format!("{}: ISA discipline OK", self.path)
        } else {
            let mut parts = Vec::new();
            if self.requires_isa_helpers() {
                if !self.saw_run_atom {
                    parts.push("missing run_atom".into());
                }
                if !self.saw_run_port {
                    parts.push("missing run_port".into());
                }
                if !self.saw_atom_trait_or_fn {
                    parts.push("missing Atom/AtomFn".into());
                }
                if !self.saw_port_trait_or_fn {
                    parts.push("missing Port/PortFn".into());
                }
            }
            for v in &self.violations {
                parts.push(format!("L{} {}: {}", v.line, v.kind, v.detail));
            }
            format!("{}: FAIL — {}", self.path, parts.join("; "))
        }
    }
}

/// Best-effort scan: flags banned I/O **outside** `impl Port` blocks.
pub fn scan_source(path: &str, source: &str) -> DisciplineReport {
    let mut report = DisciplineReport {
        path: path.to_string(),
        ..Default::default()
    };
    let composer = is_composer_bearing_source(path, source);

    let mut in_port_impl = 0i32;
    let mut brace_depth = 0i32;
    let mut port_impl_base_depth: Option<i32> = None;

    for (idx, raw_line) in source.lines().enumerate() {
        let line_no = idx + 1;
        let line = strip_line_comment(raw_line);
        let trimmed = line.trim();

        // Track braces for Port impl region.
        let opens = line.chars().filter(|c| *c == '{').count() as i32;
        let closes = line.chars().filter(|c| *c == '}').count() as i32;

        if trimmed.starts_with("impl ") && trimmed.contains("Port") && !trimmed.contains("PortFn")
        {
            in_port_impl += 1;
            port_impl_base_depth = Some(brace_depth);
        }

        brace_depth += opens - closes;

        if in_port_impl > 0 {
            if let Some(base) = port_impl_base_depth {
                // Left the impl when depth returns to base and we saw a close.
                if brace_depth <= base && closes > 0 && opens == 0 {
                    in_port_impl = in_port_impl.saturating_sub(1);
                    if in_port_impl == 0 {
                        port_impl_base_depth = None;
                    }
                }
            }
        }

        if line.contains("run_atom") {
            report.saw_run_atom = true;
        }
        if line.contains("run_port") {
            report.saw_run_port = true;
        }
        if line.contains("AtomFn") || line.contains("impl Atom") || line.contains(": Atom<") {
            report.saw_atom_trait_or_fn = true;
        }
        if line.contains("PortFn")
            || line.contains("impl Port")
            || line.contains(": Port<")
            || (line.contains("struct ") && line.to_lowercase().contains("port"))
        {
            report.saw_port_trait_or_fn = true;
        }

        // Banned side effects only outside Port impl bodies.
        if in_port_impl == 0 {
            for token in BANNED_SIDE_EFFECT_TOKENS {
                if line.contains(token) {
                    report.violations.push(DisciplineViolation {
                        line: line_no,
                        kind: "banned-side-effect".into(),
                        detail: format!("`{token}` outside Port impl"),
                    });
                }
            }
        }
    }

    // Soft structural requirements for commercial composers (path + discovery markers).
    if composer {
        if !report.saw_run_atom {
            report.violations.push(DisciplineViolation {
                line: 0,
                kind: "missing-helper".into(),
                detail: "composer must call run_atom".into(),
            });
        }
        if !report.saw_run_port {
            report.violations.push(DisciplineViolation {
                line: 0,
                kind: "missing-helper".into(),
                detail: "composer must call run_port".into(),
            });
        }
    }

    report
}

fn strip_line_comment(line: &str) -> String {
    if let Some(i) = line.find("//") {
        // Keep URLs like http://
        if i > 0 && line.as_bytes().get(i - 1) == Some(&b':') {
            return line.to_string();
        }
        line[..i].to_string()
    } else {
        line.to_string()
    }
}

/// Assert witness summaries only use legal step-kind prefixes for ISA steps.
pub fn assert_witness_step_kinds(summaries: &[String]) -> IsaResult<()> {
    let legal = [
        StepKind::Atom.as_str(),
        StepKind::Port.as_str(),
        StepKind::Adapter.as_str(),
        StepKind::Composer.as_str(),
        StepKind::Governor.as_str(),
    ];
    for s in summaries {
        let prefix = s.split(':').next().unwrap_or("");
        if prefix.is_empty() {
            continue;
        }
        // Non-ISA summaries (runtime) may exist; only check known ISA prefixes.
        if legal.iter().any(|p| prefix == *p) || s.contains(':') {
            // ok if first segment is legal OR free-form demo text after governor path
            if legal.iter().any(|p| prefix == *p) {
                continue;
            }
        }
        // Allow free-form only if it does not claim a fake step kind.
        let fake = ["service", "util", "helper", "executionstep", "execution_step"];
        if fake.iter().any(|f| prefix.eq_ignore_ascii_case(f)) {
            return Err(IsaError::atom(
                "discipline",
                format!("illegal witness step prefix `{prefix}` in `{s}`"),
            ));
        }
    }
    Ok(())
}

/// Human-readable discipline policy (for surface / docs).
pub fn policy_text() -> &'static str {
    "L4/L8: business = Atom|Port|Adapter|Composer; side effects only via Port + run_port; \
     history only via WitnessJournal; no dual ExecutionStep authority"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_composer_passes() {
        let src = r#"
use axiom_isa::{run_atom, run_port, AtomFn, Port, SeqComposer};
struct SandboxPort;
impl Port<String, String> for SandboxPort {
    fn name(&self) -> &str { "sb" }
    fn call(&mut self, input: String) -> IsaResult<String> {
        // Port may use I/O:
        let _ = std::fs::read_to_string("x");
        Ok(input)
    }
}
fn build() {
    let a = AtomFn::new("v", |x: i32| Ok(x));
    let _ = run_atom(&a, 1, journal);
    let mut p = SandboxPort;
    let _ = run_port(&mut p, "x".into(), journal);
}
"#;
        let r = scan_source("workbench.rs", src);
        assert!(r.ok(), "{}", r.summary());
    }

    #[test]
    fn banned_io_outside_port_fails() {
        let src = r#"
fn bad_atom() {
    let _ = std::fs::read_to_string("secret");
}
use run_atom;
use run_port;
struct AtomFn;
struct PortFn;
"#;
        let r = scan_source("pipeline.rs", src);
        assert!(!r.violations.is_empty(), "expected banned-side-effect");
    }

    #[test]
    fn witness_prefix_rejects_service() {
        let bad = vec!["service:do_stuff:ok".into()];
        assert!(assert_witness_step_kinds(&bad).is_err());
        let good = vec!["atom:validate:run".into(), "port:execute:ok".into()];
        assert!(assert_witness_step_kinds(&good).is_ok());
    }

    #[test]
    fn discover_includes_baseline_and_composer_markers() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..");
        let found = discover_composer_sources(&root);
        assert!(
            found.iter().any(|p| p.ends_with("pipeline.rs")),
            "discover must find pipeline.rs: {found:?}"
        );
        assert!(
            found.iter().any(|p| p.ends_with("workbench.rs")),
            "discover must find workbench.rs: {found:?}"
        );
        let bad = scan_workspace_commercial(&root);
        assert!(
            bad.is_empty(),
            "workspace commercial scan must pass: {:?}",
            bad.iter().map(|r| r.summary()).collect::<Vec<_>>()
        );
    }
}

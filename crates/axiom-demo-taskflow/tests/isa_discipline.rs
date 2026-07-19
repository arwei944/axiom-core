//! T8 — commercial path must express side effects only via Atom/Port ISA helpers.

use axiom_isa::{scan_source, COMMERCIAL_ISA_SOURCES};
use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    // tests run with CARGO_MANIFEST_DIR = crates/axiom-demo-taskflow
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

#[test]
fn commercial_sources_pass_isa_discipline() {
    let root = workspace_root();
    let mut failures = Vec::new();
    for rel in COMMERCIAL_ISA_SOURCES {
        let path = root.join(rel);
        let src = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) => {
                failures.push(format!("missing {rel}: {e}"));
                continue;
            }
        };
        let report = scan_source(rel, &src);
        if !report.ok() {
            failures.push(report.summary());
        }
    }
    assert!(
        failures.is_empty(),
        "ISA discipline violations:\n{}",
        failures.join("\n")
    );
}

#[test]
fn workbench_and_pipeline_use_run_port() {
    let root = workspace_root();
    for rel in [
        "crates/axiom-demo-taskflow/src/pipeline.rs",
        "crates/axiom-demo-taskflow/src/workbench.rs",
    ] {
        let src = std::fs::read_to_string(root.join(rel)).expect(rel);
        assert!(src.contains("run_port"), "{rel} must call run_port");
        assert!(src.contains("run_atom"), "{rel} must call run_atom");
        assert!(
            !src.contains("reqwest::"),
            "{rel} must not call reqwest outside Port discipline"
        );
    }
}

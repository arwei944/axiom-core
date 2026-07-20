//! T8 — commercial path must express side effects only via Atom/Port ISA helpers.

use axiom_isa::{
    discover_composer_sources, scan_workspace_commercial, COMMERCIAL_ISA_SOURCES,
};
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
    let discovered = discover_composer_sources(&root);
    assert!(
        discovered.len() >= COMMERCIAL_ISA_SOURCES.len(),
        "discovery must cover baseline list: {discovered:?}"
    );
    let failures = scan_workspace_commercial(&root);
    assert!(
        failures.is_empty(),
        "ISA discipline violations:\n{}",
        failures
            .iter()
            .map(|r| r.summary())
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn discovery_picks_up_composer_markers() {
    let root = workspace_root();
    let found = discover_composer_sources(&root);
    assert!(found.iter().any(|p| p.ends_with("pipeline.rs")), "{found:?}");
    assert!(found.iter().any(|p| p.ends_with("workbench.rs")), "{found:?}");
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

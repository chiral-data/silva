//! Integration tests for pre-check validation of input_files folder and cross-node references.
//!
//! These tests run silva headless against workflow fixtures and verify that
//! pre-checks reject invalid workflows before any containers are started.
//!
//! These tests do NOT require Docker since pre-checks run before execution.

use std::path::PathBuf;
use std::process::Command;

/// Returns the path to the silva binary built by cargo
fn silva_bin() -> PathBuf {
    let mut path = std::env::current_exe()
        .expect("Failed to get current exe path")
        .parent()
        .expect("Failed to get parent dir")
        .parent()
        .expect("Failed to get target dir")
        .to_path_buf();
    path.push("silva");
    path
}

/// Returns the path to a precheck test fixture workflow
fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/workflow-precheck")
        .join(name)
}

/// Runs silva on a workflow fixture and returns (success, stdout, stderr)
fn run_silva(fixture_name: &str) -> (bool, String, String) {
    let fixture = fixture_path(fixture_name);
    assert!(fixture.exists(), "Fixture not found: {}", fixture.display());

    let output = Command::new(silva_bin())
        .arg(&fixture)
        .output()
        .expect("Failed to run silva binary");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    (output.status.success(), stdout, stderr)
}

#[test]
fn test_missing_input_files_rejected() {
    let (success, stdout, stderr) = run_silva("missing-input-files");
    assert!(!success, "Workflow should be rejected by pre-check");

    let combined = format!("{stdout}{stderr}");
    assert!(
        combined.contains("No 'input_files/' folder found"),
        "Should mention missing input_files folder. Output:\n{combined}"
    );
    assert!(
        combined.contains("[01-ingest]"),
        "Should list the dependency-free job. Output:\n{combined}"
    );
}

#[test]
fn test_cross_node_reference_rejected() {
    let (success, stdout, stderr) = run_silva("cross-node-precheck");
    assert!(!success, "Workflow should be rejected by pre-check");

    let combined = format!("{stdout}{stderr}");
    assert!(
        combined.contains("Cross-node path references"),
        "Should mention cross-node references. Output:\n{combined}"
    );
    assert!(
        combined.contains("../01-produce"),
        "Should show the offending path. Output:\n{combined}"
    );
    assert!(
        combined.contains("[02-consume]"),
        "Should list the violating job. Output:\n{combined}"
    );
}

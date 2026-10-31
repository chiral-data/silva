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

/// Runs `silva run` (not the deprecated bare path) and returns (success, stdout, stderr).
fn silva_run(fixture_name: &str, json: bool) -> (bool, String, String) {
    let fixture = fixture_path(fixture_name);
    assert!(fixture.exists(), "Fixture not found: {}", fixture.display());

    let mut command = Command::new(silva_bin());
    command.arg("run").arg(&fixture);
    if json {
        command.arg("--json");
    }
    let output = command.output().expect("Failed to run silva binary");

    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

// The schema fixtures have no input_files/ either, so a refusal that names the
// version rather than the folder proves the version is checked first.

#[test]
fn test_newer_major_schema_version_refused_first() {
    let (success, stdout, stderr) = silva_run("schema-major", false);
    assert!(!success, "A 2.0 workflow must not run on a 1.x silva");

    let combined = format!("{stdout}{stderr}");
    assert!(
        combined.contains("requires format 2.0") && combined.contains("upgrade silva"),
        "Should say to upgrade. Output:\n{combined}"
    );
    assert!(
        !combined.contains("input_files"),
        "Should stop before the other prechecks. Output:\n{combined}"
    );
}

#[test]
fn test_malformed_schema_version_refused_first() {
    let (success, stdout, stderr) = silva_run("schema-malformed", false);
    assert!(!success, "A malformed schema_version must not run");

    let combined = format!("{stdout}{stderr}");
    assert!(
        combined.contains("MAJOR.MINOR"),
        "Should say how to write it. Output:\n{combined}"
    );
    assert!(
        !combined.contains("input_files"),
        "Should stop before the other prechecks. Output:\n{combined}"
    );
}

#[test]
fn test_newer_minor_schema_version_notes_and_continues() {
    let (success, stdout, stderr) = silva_run("schema-minor", false);
    let combined = format!("{stdout}{stderr}");
    assert!(
        combined.contains("targets format 1.2"),
        "Should note the newer minor. Output:\n{combined}"
    );
    assert!(
        !success && combined.contains("No 'input_files/' folder found"),
        "Should carry on to the next precheck. Output:\n{combined}"
    );

    let (_, stdout, _) = silva_run("schema-minor", true);
    assert!(
        stdout.lines().any(|line| {
            serde_json::from_str::<serde_json::Value>(line).is_ok_and(|event| {
                event["event"] == "note"
                    && event["text"]
                        .as_str()
                        .is_some_and(|t| t.contains("targets format 1.2"))
            })
        }),
        "Should arrive as a note event under --json. Stdout:\n{stdout}"
    );
}

//! Integration tests for the @complete job isolation feature.
//!
//! These tests run silva headless against real workflow fixtures with Docker containers
//! to verify that completed jobs are moved to @complete/ and cross-node path access is blocked.
//!
//! Requirements:
//! - Docker must be running
//! - alpine:latest image should be available (will be pulled if not)

use std::path::PathBuf;
use std::process::Command;

/// Returns the path to the silva binary built by cargo
fn silva_bin() -> PathBuf {
    // cargo test builds the binary in the same target directory
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

/// Returns the path to a test fixture workflow
fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/workflow-complete")
        .join(name)
}

/// Runs silva on a workflow fixture and returns (success, stdout, temp_folder_path)
fn run_silva(fixture_name: &str) -> (bool, String, Option<PathBuf>) {
    let fixture = fixture_path(fixture_name);
    assert!(fixture.exists(), "Fixture not found: {}", fixture.display());

    let output = Command::new(silva_bin())
        .arg(&fixture)
        .output()
        .expect("Failed to run silva binary");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    // Print output for debugging on failure
    if !output.status.success() {
        eprintln!("=== STDOUT ===\n{stdout}");
        eprintln!("=== STDERR ===\n{stderr}");
    }

    // Extract temp folder path from output
    // Silva prints either "Output folder: /tmp/silva-..." or "Working folder: /tmp/silva-..."
    let temp_path = stdout.lines().chain(stderr.lines()).find_map(|line| {
        let line = line.trim();
        if line.starts_with("Output folder:") || line.starts_with("Working folder:") {
            line.split(':').nth(1).map(|p| PathBuf::from(p.trim()))
        } else {
            None
        }
    });

    (output.status.success(), stdout, temp_path)
}

/// Check if Docker is available, skip test if not
fn require_docker() {
    let output = Command::new("docker").arg("info").output();
    match output {
        Ok(o) if o.status.success() => {}
        _ => {
            eprintln!("Docker is not available, skipping integration test");
            std::process::exit(0);
        }
    }
}

#[test]
fn test_completed_job_moved_to_complete() {
    require_docker();

    let (success, _stdout, temp_path) = run_silva("two-node-basic");
    assert!(success, "Workflow should succeed");

    let temp = temp_path.expect("Should have temp folder path");

    // 01-produce should be in @complete/, not in root
    assert!(
        !temp.join("01-produce").exists(),
        "01-produce should not exist in root after completion"
    );
    assert!(
        temp.join("@complete/01-produce").exists(),
        "01-produce should be in @complete/"
    );
    assert!(
        temp.join("@complete/01-produce/outputs/result.txt")
            .exists(),
        "01-produce outputs should be preserved in @complete/"
    );

    // 02-consume should also be in @complete/ (it's the last job)
    assert!(
        temp.join("@complete/02-consume").exists(),
        "02-consume should be in @complete/"
    );

    // Cleanup
    let _ = std::fs::remove_dir_all(&temp);
}

#[test]
fn test_cross_node_path_access_fails() {
    // No Docker needed — pre-check catches ../ references before execution
    let (success, stdout, _temp_path) = run_silva("cross-node-cheat");
    assert!(
        !success,
        "Workflow should fail because pre-check detects ../ references"
    );

    let stderr = {
        let fixture = fixture_path("cross-node-cheat");
        let output = std::process::Command::new(silva_bin())
            .arg(&fixture)
            .output()
            .expect("Failed to run silva binary");
        String::from_utf8_lossy(&output.stderr).to_string()
    };

    let combined = format!("{stdout}{stderr}");
    assert!(
        combined.contains("Cross-node path references"),
        "Should be caught by pre-check. Output:\n{combined}"
    );
}

#[test]
fn test_inputs_contract_still_works() {
    require_docker();

    let (success, _stdout, temp_path) = run_silva("inputs-contract");
    assert!(success, "Workflow should succeed using inputs/ contract");

    let temp = temp_path.expect("Should have temp folder path");

    // Both jobs should be in @complete/
    assert!(temp.join("@complete/01-produce").exists());
    assert!(temp.join("@complete/02-consume").exists());

    // Verify the report was actually generated with correct content
    let report = std::fs::read_to_string(temp.join("@complete/02-consume/outputs/report.txt"))
        .expect("report.txt should exist");
    assert!(
        report.contains("col1,col2"),
        "Report should contain CSV data"
    );
    assert!(report.contains("rows"), "Report should contain metadata");

    // Cleanup
    let _ = std::fs::remove_dir_all(&temp);
}

#[test]
fn test_three_node_chain_all_moved() {
    require_docker();

    let (success, _stdout, temp_path) = run_silva("three-node-chain");
    assert!(success, "Three-node workflow should succeed");

    let temp = temp_path.expect("Should have temp folder path");

    // All three should be in @complete/
    for node in &["01-produce", "02-consume", "03-final"] {
        assert!(!temp.join(node).exists(), "{node} should not exist in root");
        assert!(
            temp.join(format!("@complete/{node}")).exists(),
            "{node} should be in @complete/"
        );
    }

    // Verify data flowed through the chain correctly
    let final_output = std::fs::read_to_string(temp.join("@complete/03-final/outputs/final.txt"))
        .expect("final.txt should exist");
    assert!(final_output.contains("step1"), "Should contain step1 data");
    assert!(final_output.contains("step2"), "Should contain step2 data");
    assert!(final_output.contains("done"), "Should contain final marker");

    // Cleanup
    let _ = std::fs::remove_dir_all(&temp);
}

#[test]
fn test_failed_job_not_moved() {
    require_docker();

    let (success, _stdout, temp_path) = run_silva("failed-job");
    assert!(
        !success,
        "Workflow should fail because node 01 exits with error"
    );

    let temp = temp_path.expect("Should have temp folder path");

    // Failed job should stay in root, not moved to @complete/
    assert!(
        temp.join("01-produce").exists(),
        "Failed 01-produce should remain in root for debugging"
    );
    assert!(
        !temp.join("@complete/01-produce").exists(),
        "Failed 01-produce should NOT be in @complete/"
    );

    // 02-consume should never have run, so it stays in root too
    assert!(
        temp.join("02-consume").exists(),
        "02-consume should remain in root (never ran)"
    );
    assert!(
        !temp.join("@complete").exists(),
        "@complete/ directory should not exist at all"
    );

    // Cleanup
    let _ = std::fs::remove_dir_all(&temp);
}

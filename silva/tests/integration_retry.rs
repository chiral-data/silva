//! Integration tests for `retry` and `timeout` on a job's `run` script.
//!
//! Requirements:
//! - Docker must be running
//! - ubuntu:latest image should be available (will be pulled if not)

use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

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

/// Whether a Docker daemon is reachable; callers skip themselves when it isn't.
fn docker_available() -> bool {
    matches!(
        Command::new("docker").arg("info").output(),
        Ok(o) if o.status.success()
    )
}

fn container_count() -> usize {
    let output = Command::new("docker")
        .args(["ps", "-aq"])
        .output()
        .expect("Failed to run docker ps");
    String::from_utf8_lossy(&output.stdout).lines().count()
}

/// Runs `silva run` on a fixture: (success, combined output, temp folder, elapsed).
fn run_fixture(name: &str) -> (bool, String, Option<PathBuf>, Duration) {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/workflow-retry")
        .join(name);
    assert!(fixture.exists(), "Fixture not found: {}", fixture.display());

    let started = Instant::now();
    let output = Command::new(silva_bin())
        .arg("run")
        .arg(&fixture)
        .output()
        .expect("Failed to run silva binary");
    let elapsed = started.elapsed();

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let temp_path = combined.lines().find_map(|line| {
        let line = line.trim();
        (line.starts_with("Output folder:") || line.starts_with("Working folder:"))
            .then(|| line.split(':').nth(1).map(|p| PathBuf::from(p.trim())))
            .flatten()
    });
    (output.status.success(), combined, temp_path, elapsed)
}

#[test]
fn test_a_failed_attempt_is_retried_in_the_same_folder() {
    if !docker_available() {
        eprintln!(
            "[SKIP] test_a_failed_attempt_is_retried_in_the_same_folder — Docker not available"
        );
        return;
    }

    let (success, output, temp_path, _) = run_fixture("fails-once");
    assert!(success, "the retry should rescue the job:\n{output}");
    assert!(
        output.contains("exited with code 1 on attempt 1 of 2; retrying in 10s"),
        "{output}"
    );
    assert!(output.contains("(attempt 2 of 2)"), "{output}");

    // The second attempt read the count the first one left behind.
    let temp = temp_path.expect("Should have temp folder path");
    let attempts = std::fs::read_to_string(temp.join("@complete/01-job/attempts"))
        .expect("the job's attempt counter");
    assert_eq!(attempts.trim(), "2");
    let _ = std::fs::remove_dir_all(&temp);
}

#[test]
fn test_retries_run_out_and_the_job_fails() {
    if !docker_available() {
        eprintln!("[SKIP] test_retries_run_out_and_the_job_fails — Docker not available");
        return;
    }

    let (success, output, temp_path, _) = run_fixture("always-fails");
    assert!(!success, "the job fails every attempt:\n{output}");
    assert_eq!(
        output.matches("Executing script: run.sh").count(),
        2,
        "{output}"
    );
    assert!(
        output.contains("Script run.sh failed with exit code 3 after 2 attempts"),
        "{output}"
    );
    if let Some(temp) = temp_path {
        let _ = std::fs::remove_dir_all(&temp);
    }
}

#[test]
fn test_a_timeout_stops_the_script_and_removes_its_container() {
    if !docker_available() {
        eprintln!(
            "[SKIP] test_a_timeout_stops_the_script_and_removes_its_container — Docker not available"
        );
        return;
    }

    let before = container_count();
    let (success, output, temp_path, elapsed) = run_fixture("times-out");
    assert!(!success, "the job outlives its timeout:\n{output}");
    assert!(
        output.contains("Script run.sh timed out after 2s"),
        "{output}"
    );
    assert!(output.contains("Removed container"), "{output}");
    // `run.sh` sleeps 60 s: finishing well before that means it was stopped.
    assert!(elapsed < Duration::from_secs(30), "took {elapsed:?}");
    assert_eq!(container_count(), before, "a container was left behind");
    if let Some(temp) = temp_path {
        let _ = std::fs::remove_dir_all(&temp);
    }
}

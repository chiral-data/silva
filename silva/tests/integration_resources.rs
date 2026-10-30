//! Integration tests for `[resources]` on a job: limits reach the container,
//! and containers are reused per image and resources.
//!
//! Requirements:
//! - Docker must be running, on a cgroup v2 host
//! - ubuntu:latest image should be available (will be pulled if not)

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

/// Whether a Docker daemon is reachable; callers skip themselves when it isn't.
fn docker_available() -> bool {
    matches!(
        Command::new("docker").arg("info").output(),
        Ok(o) if o.status.success()
    )
}

/// Runs `silva run` on a fixture: (success, combined output).
fn run_fixture(name: &str) -> (bool, String) {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/workflow-resources")
        .join(name);
    assert!(fixture.exists(), "Fixture not found: {}", fixture.display());

    let output = Command::new(silva_bin())
        .arg("run")
        .arg(&fixture)
        .output()
        .expect("Failed to run silva binary");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    if let Some(temp) = combined.lines().find_map(|line| {
        let line = line.trim();
        (line.starts_with("Output folder:") || line.starts_with("Working folder:"))
            .then(|| line.split(':').nth(1).map(|p| PathBuf::from(p.trim())))
            .flatten()
    }) {
        let _ = std::fs::remove_dir_all(temp);
    }
    (output.status.success(), combined)
}

#[test]
fn test_cpu_and_memory_limits_reach_the_container() {
    if !docker_available() {
        eprintln!("[SKIP] test_cpu_and_memory_limits_reach_the_container — Docker not available");
        return;
    }

    let (success, output) = run_fixture("limits-reach-the-container");
    assert!(success, "{output}");
    // Read from inside the container: cpus = 0.5, memory = "64m", no swap on top.
    assert!(output.contains("cpu.max: 50000 100000"), "{output}");
    assert!(output.contains("memory.max: 67108864"), "{output}");
    // A host without swap accounting has no such file; silva then logs the dropped cap.
    if output.contains("memory.swap.max:") {
        assert!(output.contains("memory.swap.max: 0"), "{output}");
    }
}

#[test]
fn test_jobs_with_different_resources_get_their_own_container() {
    if !docker_available() {
        eprintln!(
            "[SKIP] test_jobs_with_different_resources_get_their_own_container — Docker not available"
        );
        return;
    }

    let (success, output) = run_fixture("split-by-resources");
    assert!(success, "{output}");
    // 01-plain and 02-limited differ; 03-same-limit shares 02-limited's container.
    assert_eq!(
        output.matches("Creating container with image").count(),
        2,
        "{output}"
    );
    assert_eq!(
        output.matches("Reusing existing container").count(),
        1,
        "{output}"
    );
}

#[test]
fn test_jobs_without_resources_still_share_one_container() {
    if !docker_available() {
        eprintln!(
            "[SKIP] test_jobs_without_resources_still_share_one_container — Docker not available"
        );
        return;
    }

    let (success, output) = run_fixture("no-resources-reuses");
    assert!(success, "{output}");
    assert_eq!(
        output.matches("Creating container with image").count(),
        1,
        "{output}"
    );
    assert_eq!(
        output.matches("Reusing existing container").count(),
        1,
        "{output}"
    );
}

//! Integration tests for building the local app images a workflow ships in `apps/`.
//!
//! These run silva headless against real fixtures with a real Docker daemon, and
//! cover the negative cases as well as the happy path: an app directory nothing
//! references must not be built, a registry-qualified image must not be treated
//! as a local app, a second run must not rebuild, and a failing build must abort
//! before any container starts.
//!
//! Requirements:
//! - Docker must be running
//! - The fixtures build `FROM ubuntu:latest`, pulled if absent. Ubuntu rather
//!   than alpine because silva execs `/bin/bash`, which alpine does not have.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Returns the path to the silva binary built by cargo.
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

/// Returns the path to a fixture in the `workflow-apps` family.
fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/workflow-apps")
        .join(name)
}

/// Returns the path to a fixture in the `workflow-complete` family, reused here
/// for the "workflow ships no apps/" case rather than duplicating a fixture.
fn complete_fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/workflow-complete")
        .join(name)
}

/// Runs silva on a fixture and returns (success, stdout and stderr combined).
///
/// Uses the `run` subcommand rather than the bare positional path, which silva
/// now warns is deprecated.
fn run_silva(fixture: &Path) -> (bool, String) {
    assert!(fixture.exists(), "Fixture not found: {}", fixture.display());

    let output = Command::new(silva_bin())
        .arg("run")
        .arg(fixture)
        .output()
        .expect("Failed to run silva binary");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    if !output.status.success() {
        eprintln!("=== silva output ===\n{combined}");
    }

    (output.status.success(), combined)
}

/// Whether a Docker daemon is reachable. Callers must branch on this and return
/// themselves -- see the note in `integration_complete.rs` for why this is not
/// an `exit(0)`.
fn docker_available() -> bool {
    matches!(
        Command::new("docker").arg("info").output(),
        Ok(o) if o.status.success()
    )
}

/// The image ID for `tag`, or `None` when no such image exists locally.
fn docker_image_id(tag: &str) -> Option<String> {
    let output = Command::new("docker")
        .args(["images", "-q", tag])
        .output()
        .ok()?;
    let id = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if id.is_empty() { None } else { Some(id) }
}

fn remove_image(tag: &str) {
    let _ = Command::new("docker").args(["rmi", "-f", tag]).output();
}

const APP: &str = "silva-selftest:2026_08_27";
const UNUSED: &str = "silva-unused:2026_01_01";
const FAILING: &str = "silva-selftest-fail:2026_08_27";

#[test]
fn test_local_apps_built_used_and_skipped_on_rerun() {
    if !docker_available() {
        eprintln!("[SKIP] test_local_apps_built_used_and_skipped_on_rerun — Docker not available");
        return;
    }

    // Start cold, so the build path is actually exercised rather than skipped.
    remove_image(APP);
    remove_image(UNUSED);
    assert!(
        docker_image_id(APP).is_none(),
        "{APP} should not exist before the run"
    );

    let fixture = fixture_path("local-app");
    let (success, out) = run_silva(&fixture);
    assert!(success, "workflow should succeed");

    // Exactly the one referenced app was picked up.
    assert!(
        out.contains(&format!("Workflow ships 1 local app image(s): {APP}")),
        "pre-flight should report the one referenced app"
    );
    assert!(
        out.contains("Building image from"),
        "the app image should have been built"
    );
    assert!(
        out.contains(&format!("Built {APP}")),
        "the build should report the full tag, not :latest"
    );

    // The job ran *on* the built image: this marker only exists inside it.
    assert!(
        out.contains("silva-selftest-marker"),
        "the job should have run on the newly built image"
    );

    let built = docker_image_id(APP).expect("app image should exist after the run");

    // The unreferenced app was skipped. Its Dockerfile exits 7, so an attempt to
    // build it would have failed the whole run rather than passing quietly.
    assert!(
        docker_image_id(UNUSED).is_none(),
        "{UNUSED} is referenced by no job and must not be built"
    );
    assert!(
        !out.contains("silva-unused"),
        "the unreferenced app should not appear in the log at all"
    );

    // The registry-qualified image was left to the existing pull path.
    assert!(
        out.contains("Image already exists locally: ubuntu:latest")
            || out.contains("Pulling image: ubuntu:latest"),
        "ubuntu:latest should go through pull_image, not the build path"
    );

    // A second run must skip the build entirely and leave the image untouched.
    let (success, out) = run_silva(&fixture);
    assert!(success, "the second run should also succeed");
    assert!(
        out.contains(&format!("Local app image already built, skipping: {APP}")),
        "the second run should skip the build"
    );
    assert!(
        !out.contains("Building image from"),
        "the second run must not rebuild"
    );
    assert_eq!(
        docker_image_id(APP).as_deref(),
        Some(built.as_str()),
        "the image should be byte-for-byte the same one, not rebuilt"
    );

    remove_image(APP);
}

#[test]
fn test_failing_app_build_aborts_before_any_container() {
    if !docker_available() {
        eprintln!(
            "[SKIP] test_failing_app_build_aborts_before_any_container — Docker not available"
        );
        return;
    }

    remove_image(FAILING);

    let (success, out) = run_silva(&fixture_path("failing-app"));
    assert!(!success, "a failing app build must fail the workflow");

    // Docker's own build output reaches the log, so the failure is diagnosable.
    assert!(
        out.contains("deliberate failure"),
        "the build's own output should be streamed through"
    );

    // And the summary carries what Docker actually said. bollard's Display for
    // its stream error is the literal string "Docker stream error", which throws
    // the useful part away.
    assert!(
        out.contains("returned a non-zero code: 3"),
        "the real docker error should be recovered, not swallowed"
    );
    assert!(
        !out.contains("Docker stream error"),
        "the discarded-message bug should not have regressed"
    );

    // Nothing should have started.
    assert!(
        !out.contains("Creating container"),
        "no container may be created when the app build fails"
    );

    assert!(
        docker_image_id(FAILING).is_none(),
        "a failed build must leave no tagged image"
    );
}

#[test]
fn test_workflow_without_apps_folder_is_unaffected() {
    if !docker_available() {
        eprintln!("[SKIP] test_workflow_without_apps_folder_is_unaffected — Docker not available");
        return;
    }

    let (success, out) = run_silva(&complete_fixture_path("two-node-basic"));
    assert!(success, "a workflow without apps/ should still run");
    assert!(
        !out.contains("local app image"),
        "a workflow with no apps/ folder should produce no pre-flight output"
    );
    assert!(
        !out.contains("Building image from"),
        "a workflow with no apps/ folder should build nothing"
    );
}

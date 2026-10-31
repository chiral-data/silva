//! Integration tests for `silva export --cwl`: the export is valid CWL, and
//! cwltool running it produces the same per-job outputs as `silva run`.
//!
//! Requirements: Docker and `cwltool` on PATH. Each test skips itself when
//! either is missing; CI installs both.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

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

fn available(program: &str, arg: &str) -> bool {
    matches!(Command::new(program).arg(arg).output(), Ok(o) if o.status.success())
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/workflow-complete")
        .join(name)
}

/// Every file under `dir`, by relative path, with its content.
fn tree(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut files = BTreeMap::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        for entry in std::fs::read_dir(&current).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                let rel = path
                    .strip_prefix(dir)
                    .unwrap()
                    .to_string_lossy()
                    .to_string();
                files.insert(rel, std::fs::read(&path).unwrap());
            }
        }
    }
    files
}

/// Exports `name`, runs it with cwltool and with `silva run`, and compares
/// each job's outputs.
fn export_and_compare(name: &str, jobs: &[&str]) {
    let scratch = tempfile::tempdir().unwrap();
    let cwl_dir = scratch.path().join("cwl");
    let export = Command::new(silva_bin())
        .args(["export", "--cwl"])
        .arg(fixture(name))
        .arg("-o")
        .arg(&cwl_dir)
        .output()
        .unwrap();
    assert!(
        export.status.success(),
        "{}",
        String::from_utf8_lossy(&export.stderr)
    );

    let workflow = cwl_dir.join("workflow.cwl");
    let validate = Command::new("cwltool")
        .arg("--validate")
        .arg(&workflow)
        .output()
        .unwrap();
    assert!(
        validate.status.success(),
        "{}",
        String::from_utf8_lossy(&validate.stderr)
    );

    let cwl_out = scratch.path().join("cwl-out");
    let run = Command::new("cwltool")
        .arg("--outdir")
        .arg(&cwl_out)
        .arg(&workflow)
        .output()
        .unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );

    let silva = Command::new(silva_bin())
        .arg("run")
        .arg(fixture(name))
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&silva.stdout).to_string();
    assert!(silva.status.success(), "{stdout}");
    let silva_dir = stdout
        .lines()
        .find_map(|l| {
            l.trim()
                .strip_prefix("Output folder:")
                .map(|p| PathBuf::from(p.trim()))
        })
        .expect("silva prints its output folder");

    for job in jobs {
        let from_silva = tree(&silva_dir.join("@complete").join(job).join("outputs"));
        let from_cwl = tree(&cwl_out.join(job));
        assert!(!from_silva.is_empty(), "{job}: silva collected nothing");
        assert_eq!(from_cwl, from_silva, "{job}: outputs differ");
    }
    let _ = std::fs::remove_dir_all(&silva_dir);
}

fn tools_available(test: &str) -> bool {
    let ok = available("docker", "info") && available("cwltool", "--version");
    if !ok {
        eprintln!("[SKIP] {test} — needs Docker and cwltool");
    }
    ok
}

#[test]
fn test_two_node_export_matches_silva_run() {
    if tools_available("test_two_node_export_matches_silva_run") {
        export_and_compare("two-node-basic", &["01-produce", "02-consume"]);
    }
}

#[test]
fn test_brace_pattern_export_matches_silva_run() {
    if tools_available("test_brace_pattern_export_matches_silva_run") {
        export_and_compare("brace-glob", &["01-produce", "02-consume"]);
    }
}

#[test]
fn test_inputs_contract_export_matches_silva_run() {
    if tools_available("test_inputs_contract_export_matches_silva_run") {
        export_and_compare("inputs-contract", &["01-produce", "02-consume"]);
    }
}

#[test]
fn test_an_image_file_is_refused() {
    let scratch = tempfile::tempdir().unwrap();
    let wf = scratch.path().join("wf");
    let mut options = fs_extra::dir::CopyOptions::new();
    options.copy_inside = true;
    fs_extra::dir::copy(fixture("two-node-basic"), &wf, &options).unwrap();
    let toml = wf.join("02-consume/.chiral/job.toml");
    let text = std::fs::read_to_string(&toml)
        .unwrap()
        .replace("ubuntu:latest", "consume.sif");
    std::fs::write(&toml, text).unwrap();

    let out = Command::new(silva_bin())
        .args(["export", "--cwl"])
        .arg(&wf)
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("02-consume: image \"consume.sif\" is an image file"),
        "{stderr}"
    );
    assert!(!wf.join("cwl").exists(), "a refused export writes nothing");
}

//! Integration tests for `silva export`: the export is valid CWL or WDL, and
//! cwltool or miniwdl running it produces the same per-job outputs as `silva run`.
//!
//! Requirements: Docker, plus `cwltool` or `miniwdl` on PATH. Each test skips
//! itself when its tools are missing; CI installs all three. The first
//! `miniwdl run` turns on a single-node Docker swarm, which miniwdl runs tasks in.

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

#[derive(Clone, Copy)]
enum Target {
    Cwl,
    Wdl,
}

impl Target {
    fn flag(self) -> &'static str {
        match self {
            Target::Cwl => "--cwl",
            Target::Wdl => "--wdl",
        }
    }

    fn tool(self) -> &'static str {
        match self {
            Target::Cwl => "cwltool",
            Target::Wdl => "miniwdl",
        }
    }

    fn folder(self) -> &'static str {
        match self {
            Target::Cwl => "cwl",
            Target::Wdl => "wdl",
        }
    }
}

/// Checks the export with its language's tool, runs it, and returns each
/// job's output folder by job name.
fn run_export(target: Target, dir: &Path, scratch: &Path) -> BTreeMap<String, PathBuf> {
    let (check, run) = match target {
        Target::Cwl => {
            let workflow = dir.join("workflow.cwl");
            let out = scratch.join("cwl-out");
            let mut check = Command::new("cwltool");
            check.arg("--validate").arg(&workflow);
            let mut run = Command::new("cwltool");
            run.arg("--outdir").arg(&out).arg(&workflow);
            (check, run)
        }
        Target::Wdl => {
            let workflow = dir.join("workflow.wdl");
            let mut check = Command::new("miniwdl");
            check.arg("check").arg(&workflow);
            let mut run = Command::new("miniwdl");
            run.arg("run")
                .arg(&workflow)
                .arg("-i")
                .arg(dir.join("inputs.json"))
                .arg("--dir")
                .arg(scratch.join("wdl-run"));
            (check, run)
        }
    };
    for (what, mut command) in [("check", check), ("run", run)] {
        let out = command.output().unwrap();
        assert!(
            out.status.success(),
            "{} {what} failed:\n{}",
            target.tool(),
            String::from_utf8_lossy(&out.stderr)
        );
        if what != "run" {
            continue;
        }
        return match target {
            Target::Cwl => std::fs::read_dir(scratch.join("cwl-out"))
                .unwrap()
                .flatten()
                .map(|e| (e.file_name().to_string_lossy().to_string(), e.path()))
                .collect(),
            // miniwdl prints its outputs as JSON; each is a folder named after its job.
            Target::Wdl => {
                let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
                report["outputs"]
                    .as_object()
                    .unwrap()
                    .values()
                    .map(|v| {
                        let path = PathBuf::from(v.as_str().unwrap());
                        let job = path.file_name().unwrap().to_string_lossy().to_string();
                        (job, path)
                    })
                    .collect()
            }
        };
    }
    unreachable!()
}

/// Exports `name`, runs it with the target's tool and with `silva run`, and
/// compares each job's outputs.
fn export_and_compare(target: Target, name: &str, jobs: &[&str]) {
    let scratch = tempfile::tempdir().unwrap();
    let dir = scratch.path().join(target.folder());
    let export = Command::new(silva_bin())
        .args(["export", target.flag()])
        .arg(fixture(name))
        .arg("-o")
        .arg(&dir)
        .output()
        .unwrap();
    assert!(
        export.status.success(),
        "{}",
        String::from_utf8_lossy(&export.stderr)
    );
    let exported = run_export(target, &dir, scratch.path());

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
        let from_export = tree(&exported[*job]);
        assert!(!from_silva.is_empty(), "{job}: silva collected nothing");
        assert_eq!(from_export, from_silva, "{job}: outputs differ");
    }
    let _ = std::fs::remove_dir_all(&silva_dir);
}

fn tools_available(target: Target, test: &str) -> bool {
    let ok = available("docker", "info") && available(target.tool(), "--version");
    if !ok {
        eprintln!("[SKIP] {test} — needs Docker and {}", target.tool());
    }
    ok
}

#[test]
fn test_two_node_export_matches_silva_run() {
    if tools_available(Target::Cwl, "test_two_node_export_matches_silva_run") {
        export_and_compare(Target::Cwl, "two-node-basic", &["01-produce", "02-consume"]);
    }
}

#[test]
fn test_brace_pattern_export_matches_silva_run() {
    if tools_available(Target::Cwl, "test_brace_pattern_export_matches_silva_run") {
        export_and_compare(Target::Cwl, "brace-glob", &["01-produce", "02-consume"]);
    }
}

#[test]
fn test_inputs_contract_export_matches_silva_run() {
    if tools_available(Target::Cwl, "test_inputs_contract_export_matches_silva_run") {
        export_and_compare(
            Target::Cwl,
            "inputs-contract",
            &["01-produce", "02-consume"],
        );
    }
}

#[test]
fn test_two_node_wdl_export_matches_silva_run() {
    if tools_available(Target::Wdl, "test_two_node_wdl_export_matches_silva_run") {
        export_and_compare(Target::Wdl, "two-node-basic", &["01-produce", "02-consume"]);
    }
}

#[test]
fn test_brace_pattern_wdl_export_matches_silva_run() {
    if tools_available(
        Target::Wdl,
        "test_brace_pattern_wdl_export_matches_silva_run",
    ) {
        export_and_compare(Target::Wdl, "brace-glob", &["01-produce", "02-consume"]);
    }
}

#[test]
fn test_inputs_contract_wdl_export_matches_silva_run() {
    if tools_available(
        Target::Wdl,
        "test_inputs_contract_wdl_export_matches_silva_run",
    ) {
        export_and_compare(
            Target::Wdl,
            "inputs-contract",
            &["01-produce", "02-consume"],
        );
    }
}

/// A copy of `two-node-basic` with `02-consume/.chiral/job.toml` edited.
fn edited_copy(scratch: &Path, edit: impl Fn(String) -> String) -> PathBuf {
    let wf = scratch.join("wf");
    let mut options = fs_extra::dir::CopyOptions::new();
    options.copy_inside = true;
    fs_extra::dir::copy(fixture("two-node-basic"), &wf, &options).unwrap();
    let toml = wf.join("02-consume/.chiral/job.toml");
    let text = std::fs::read_to_string(&toml).unwrap();
    std::fs::write(&toml, edit(text)).unwrap();
    wf
}

#[test]
fn test_an_image_file_is_refused() {
    for target in [Target::Cwl, Target::Wdl] {
        let scratch = tempfile::tempdir().unwrap();
        let wf = edited_copy(scratch.path(), |t| {
            t.replace("ubuntu:latest", "consume.sif")
        });
        let out = Command::new(silva_bin())
            .args(["export", target.flag()])
            .arg(&wf)
            .output()
            .unwrap();
        assert!(!out.status.success());
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains("02-consume: image \"consume.sif\" is an image file"),
            "{stderr}"
        );
        assert!(
            !wf.join(target.folder()).exists(),
            "a refused export writes nothing"
        );
    }
}

#[test]
fn test_a_param_wdl_cannot_name_is_refused() {
    let scratch = tempfile::tempdir().unwrap();
    let wf = edited_copy(scratch.path(), |t| {
        t + "\n[params.my-mode]\ntype = \"string\"\ndefault = \"fast\"\nhint = \"h\"\n"
    });
    let out = Command::new(silva_bin())
        .args(["export", "--wdl"])
        .arg(&wf)
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("02-consume: param my-mode becomes PARAM_MY-MODE"),
        "{stderr}"
    );
    assert!(!wf.join("wdl").exists(), "a refused export writes nothing");
}

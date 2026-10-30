//! Pre-execution checks for workflow scripts.
//!
//! Validates workflow conventions before any containers are started:
//! - Rejects scripts containing install commands (should be in Docker images)
//! - Requires `input_files/` folder when dependency-free jobs exist
//! - Rejects scripts with cross-node `../` path references
//!
//! Each script check has a collector, which `silva validate` reports one
//! finding per offence from, and a `check_*` that folds them into one message.

use std::fs;
use std::path::Path;

use crate::components::workflow::JobFolder;

/// A single violation found in a script.
pub(crate) struct Violation {
    pub job_name: String,
    pub script_name: String,
    pub line_number: usize,
    pub line_content: String,
}

/// What every report of one script convention says around its offences.
#[derive(Debug, PartialEq)]
pub(crate) struct ScriptConvention {
    pub heading: &'static str,
    pub fix: &'static str,
}

pub(crate) const INSTALL_COMMANDS: ScriptConvention = ScriptConvention {
    heading: "Install commands found in workflow scripts.\n\
              Dependencies must be pre-baked into Docker images.",
    fix: "Move install commands into Dockerfiles and rebuild the images.",
};

pub(crate) const CROSS_NODE_REFERENCES: ScriptConvention = ScriptConvention {
    heading: "Cross-node path references found in workflow scripts.\n\
              Jobs must use the inputs/ folder, not relative paths to siblings.",
    fix: "Use files from the inputs/ directory instead of ../",
};

pub(crate) const RUN_SCRIPTS: ScriptConvention = ScriptConvention {
    heading: "Run script not found:",
    fix: "Add the script, or point [scripts] run at the one the job uses.",
};

/// Install command patterns to detect: (command, subcommand).
const INSTALL_PATTERNS: &[(&str, &str)] = &[
    ("pip", "install"),
    ("pip3", "install"),
    ("apt-get", "install"),
    ("apt", "install"),
    ("conda", "install"),
    ("npm", "install"),
    ("apk", "add"),
];

/// Checks whether a line contains an install command by tokenizing on whitespace.
fn line_has_install_command(line: &str) -> bool {
    let tokens: Vec<&str> = line.split_whitespace().collect();
    for window in tokens.windows(2) {
        for &(cmd, subcmd) in INSTALL_PATTERNS {
            if window[0] == cmd && window[1] == subcmd {
                return true;
            }
        }
    }
    false
}

/// Every script line that installs a package, across all jobs.
pub(crate) fn install_command_violations(jobs: &[JobFolder]) -> Vec<Violation> {
    scan_jobs(jobs, line_has_install_command)
}

/// Checks all job scripts for install commands.
///
/// Returns `Ok(())` if no install commands are found, or `Err(message)`
/// with a formatted error listing all violations.
pub fn check_install_commands(jobs: &[JobFolder]) -> Result<(), String> {
    violations_message(&INSTALL_COMMANDS, &install_command_violations(jobs))
}

/// Every script line that reaches a sibling job through `../`, across all jobs.
pub(crate) fn cross_node_violations(jobs: &[JobFolder]) -> Vec<Violation> {
    scan_jobs(jobs, |line| line.contains("../"))
}

/// Checks all job scripts for cross-node `../` path references.
///
/// Jobs must use their `inputs/` folder instead of relative paths to siblings.
/// Returns `Err` listing all violations if any are found.
pub fn check_cross_node_references(jobs: &[JobFolder]) -> Result<(), String> {
    violations_message(&CROSS_NODE_REFERENCES, &cross_node_violations(jobs))
}

fn violations_message(
    convention: &ScriptConvention,
    violations: &[Violation],
) -> Result<(), String> {
    if violations.is_empty() {
        return Ok(());
    }

    let mut msg = format!("{}\n", convention.heading);
    for v in violations {
        msg.push_str(&format!(
            "\n  [{}] {}:{} — {}",
            v.job_name, v.script_name, v.line_number, v.line_content
        ));
    }
    msg.push_str(&format!("\n\nFix: {}", convention.fix));

    Err(msg)
}

/// The job's configured scripts, or the defaults when its job.toml won't load.
fn script_names(job: &JobFolder) -> Vec<String> {
    match job.load_meta() {
        Ok(meta) => vec![meta.scripts.pre, meta.scripts.run, meta.scripts.post],
        Err(_) => vec![
            "pre_run.sh".to_string(),
            "run.sh".to_string(),
            "post_run.sh".to_string(),
        ],
    }
}

/// Scans every job's scripts, line by line, skipping comments.
fn scan_jobs(jobs: &[JobFolder], offends: impl Fn(&str) -> bool) -> Vec<Violation> {
    let mut violations = Vec::new();

    for job in jobs {
        for script_name in script_names(job) {
            let Ok(content) = fs::read_to_string(job.path.join(&script_name)) else {
                continue;
            };
            for (line_idx, line) in content.lines().enumerate() {
                let trimmed = line.trim();
                if !trimmed.starts_with('#') && offends(trimmed) {
                    violations.push(Violation {
                        job_name: job.name.clone(),
                        script_name: script_name.clone(),
                        line_number: line_idx + 1,
                        line_content: trimmed.to_string(),
                    });
                }
            }
        }
    }

    violations
}

/// Checks that an `input_files/` folder exists at the workflow root when
/// there are dependency-free jobs that need input data.
///
/// Returns `Err` if dependency-free jobs exist but `input_files/` is missing.
pub fn check_input_files_folder(
    workflow_path: &Path,
    jobs: &[JobFolder],
    workflow_metadata: &job_config::workflow::WorkflowMeta,
) -> Result<(), String> {
    let jobs_without_deps: Vec<&str> = jobs
        .iter()
        .filter(|job| workflow_metadata.get_job_dependencies(&job.name).is_empty())
        .map(|job| job.name.as_str())
        .collect();

    if jobs_without_deps.is_empty() {
        return Ok(());
    }

    let input_files_path = workflow_path.join("input_files");
    if input_files_path.is_dir() {
        return Ok(());
    }

    let job_list = jobs_without_deps
        .iter()
        .map(|name| format!("[{name}]"))
        .collect::<Vec<_>>()
        .join(", ");

    Err(format!(
        "No 'input_files/' folder found, but dependency-free jobs exist:\n\
         \n  {job_list}\n\n\
         Fix: Create {}/input_files/ with the data these jobs need.",
        workflow_path.display()
    ))
}

/// Every job whose relative `run` script is missing, as (job, script).
pub(crate) fn missing_run_scripts(jobs: &[JobFolder]) -> Vec<(String, String)> {
    jobs.iter()
        .filter_map(|job| {
            let run = job.load_meta().ok()?.scripts.run;
            let relative = !Path::new(&run).is_absolute();
            (relative && !job.path.join(&run).is_file()).then(|| (job.name.clone(), run))
        })
        .collect()
}

/// Checks that every job's `run` script exists.
///
/// A missing `run` script is otherwise invisible: the script is piped into
/// bash, which reads nothing and exits 0, so the job "succeeds" doing nothing.
/// Absolute paths name a file inside the image and are not checked. Missing
/// `pre`/`post` scripts are a legitimate skip.
pub fn check_run_scripts(jobs: &[JobFolder]) -> Result<(), String> {
    let missing: Vec<String> = missing_run_scripts(jobs)
        .into_iter()
        .map(|(job, run)| format!("[{job}] {run}"))
        .collect();

    if missing.is_empty() {
        return Ok(());
    }

    Err(format!(
        "{}\n\n  {}\n\nFix: {}",
        RUN_SCRIPTS.heading,
        missing.join("\n  "),
        RUN_SCRIPTS.fix
    ))
}

/// Checks the declared format version, the same way for `validate`, `run` and
/// the TUI. A newer major may change what fields mean, so it is refused; a
/// newer minor only adds fields an older silva can skip, so it is a note.
///
/// Returns `Err` to refuse, `Ok(Some(note))` to run with a note, `Ok(None)` otherwise.
pub fn check_schema_version(
    metadata: &job_config::workflow::WorkflowMeta,
) -> Result<Option<String>, String> {
    use job_config::workflow::{SCHEMA_VERSION, SchemaCompat};

    let major = SCHEMA_VERSION.split('.').next().unwrap_or(SCHEMA_VERSION);
    match metadata.schema_compat() {
        SchemaCompat::Supported => Ok(None),
        SchemaCompat::NewerMinor(v) => Ok(Some(format!(
            "workflow targets format {v}; this silva speaks {SCHEMA_VERSION}, \
             so features newer than {SCHEMA_VERSION} are ignored."
        ))),
        SchemaCompat::UnsupportedMajor(v) => Err(format!(
            "workflow requires format {v}; this silva speaks {major}.x — upgrade silva."
        )),
        SchemaCompat::Malformed(v) => Err(format!(
            "schema_version \"{v}\" is not a format version; write MAJOR.MINOR, e.g. \"{SCHEMA_VERSION}\"."
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_job(base: &Path, name: &str, run_sh: &str) -> JobFolder {
        let job_dir = base.join(name);
        let chiral_dir = job_dir.join(".chiral");
        fs::create_dir_all(&chiral_dir).unwrap();

        fs::write(
            chiral_dir.join("job.toml"),
            format!(
                r#"name = "{name}"
description = "test"

[container]
image = "python:3.11-slim"

[scripts]
run = "run.sh"
"#
            ),
        )
        .unwrap();

        fs::write(job_dir.join("run.sh"), run_sh).unwrap();

        JobFolder::new(name.to_string(), job_dir)
    }

    #[test]
    fn test_clean_scripts_pass() {
        let temp = TempDir::new().unwrap();
        let job = create_job(temp.path(), "01-clean", "#!/bin/bash\npython main.py\n");
        assert!(check_install_commands(&[job]).is_ok());
    }

    #[test]
    fn test_pip_install_detected() {
        let temp = TempDir::new().unwrap();
        let job = create_job(
            temp.path(),
            "01-bad",
            "#!/bin/bash\npip install pandas\npython main.py\n",
        );
        let err = check_install_commands(&[job]).unwrap_err();
        assert!(err.contains("pip install pandas"));
        assert!(err.contains("[01-bad]"));
        assert!(err.contains("run.sh:2"));
    }

    #[test]
    fn test_pip_install_multiple_spaces() {
        let temp = TempDir::new().unwrap();
        let job = create_job(
            temp.path(),
            "01-spaces",
            "#!/bin/bash\npip   install   pandas\n",
        );
        let err = check_install_commands(&[job]).unwrap_err();
        assert!(err.contains("[01-spaces]"));
    }

    #[test]
    fn test_pip_install_with_tabs() {
        let temp = TempDir::new().unwrap();
        let job = create_job(
            temp.path(),
            "01-tabs",
            "#!/bin/bash\npip\t\tinstall\tpandas\n",
        );
        let err = check_install_commands(&[job]).unwrap_err();
        assert!(err.contains("[01-tabs]"));
    }

    #[test]
    fn test_apt_get_detected() {
        let temp = TempDir::new().unwrap();
        let job = create_job(
            temp.path(),
            "01-apt",
            "#!/bin/bash\napt-get install -y curl\n",
        );
        let err = check_install_commands(&[job]).unwrap_err();
        assert!(err.contains("apt-get install"));
    }

    #[test]
    fn test_apk_add_detected() {
        let temp = TempDir::new().unwrap();
        let job = create_job(temp.path(), "01-apk", "#!/bin/bash\napk  add  git\n");
        let err = check_install_commands(&[job]).unwrap_err();
        assert!(err.contains("[01-apk]"));
    }

    #[test]
    fn test_comments_ignored() {
        let temp = TempDir::new().unwrap();
        let job = create_job(
            temp.path(),
            "01-commented",
            "#!/bin/bash\n# pip install pandas\npython main.py\n",
        );
        assert!(check_install_commands(&[job]).is_ok());
    }

    #[test]
    fn test_multiple_violations_listed() {
        let temp = TempDir::new().unwrap();
        let job1 = create_job(temp.path(), "01-a", "#!/bin/bash\npip install pandas\n");
        let job2 = create_job(temp.path(), "02-b", "#!/bin/bash\nconda install numpy\n");
        let err = check_install_commands(&[job1, job2]).unwrap_err();
        assert!(err.contains("[01-a]"));
        assert!(err.contains("[02-b]"));
        assert!(err.contains("conda install"));
    }

    // --- input_files folder checks ---

    fn create_workflow_meta_no_deps() -> job_config::workflow::WorkflowMeta {
        job_config::workflow::WorkflowMeta::new("test".to_string(), "test workflow".to_string())
    }

    fn create_workflow_meta_with_deps(
        deps: Vec<(&str, Vec<&str>)>,
    ) -> job_config::workflow::WorkflowMeta {
        let mut meta = job_config::workflow::WorkflowMeta::new(
            "test".to_string(),
            "test workflow".to_string(),
        );
        for (job, dep_list) in deps {
            meta.set_job_dependencies(
                job.to_string(),
                dep_list.into_iter().map(String::from).collect(),
            );
        }
        meta
    }

    #[test]
    fn test_input_files_present_passes() {
        let temp = TempDir::new().unwrap();
        fs::create_dir(temp.path().join("input_files")).unwrap();
        let job = create_job(temp.path(), "01-ingest", "#!/bin/bash\npython main.py\n");
        let meta = create_workflow_meta_no_deps();
        assert!(check_input_files_folder(temp.path(), &[job], &meta).is_ok());
    }

    #[test]
    fn test_input_files_missing_errors() {
        let temp = TempDir::new().unwrap();
        let job = create_job(temp.path(), "01-ingest", "#!/bin/bash\npython main.py\n");
        let meta = create_workflow_meta_no_deps();
        let err = check_input_files_folder(temp.path(), &[job], &meta).unwrap_err();
        assert!(err.contains("No 'input_files/' folder found"));
        assert!(err.contains("[01-ingest]"));
    }

    #[test]
    fn test_input_files_not_needed_when_all_have_deps() {
        let temp = TempDir::new().unwrap();
        let job = create_job(temp.path(), "02-analysis", "#!/bin/bash\npython main.py\n");
        let meta = create_workflow_meta_with_deps(vec![("02-analysis", vec!["01-prep"])]);
        // No input_files/ folder, but all jobs have dependencies — should pass
        assert!(check_input_files_folder(temp.path(), &[job], &meta).is_ok());
    }

    #[test]
    fn test_input_files_missing_lists_multiple_jobs() {
        let temp = TempDir::new().unwrap();
        let job1 = create_job(temp.path(), "01-a", "#!/bin/bash\npython main.py\n");
        let job2 = create_job(temp.path(), "01-b", "#!/bin/bash\npython main.py\n");
        let meta = create_workflow_meta_no_deps();
        let err = check_input_files_folder(temp.path(), &[job1, job2], &meta).unwrap_err();
        assert!(err.contains("[01-a]"));
        assert!(err.contains("[01-b]"));
    }

    // --- cross-node reference checks ---

    #[test]
    fn test_no_cross_node_refs_passes() {
        let temp = TempDir::new().unwrap();
        let job = create_job(
            temp.path(),
            "02-analysis",
            "#!/bin/bash\ncp inputs/data.csv .\npython main.py\n",
        );
        assert!(check_cross_node_references(&[job]).is_ok());
    }

    #[test]
    fn test_cross_node_ref_detected() {
        let temp = TempDir::new().unwrap();
        let job = create_job(
            temp.path(),
            "02-analysis",
            "#!/bin/bash\ncp ../01-prep/outputs/result.csv .\npython main.py\n",
        );
        let err = check_cross_node_references(&[job]).unwrap_err();
        assert!(err.contains("[02-analysis]"));
        assert!(err.contains("../01-prep/outputs/result.csv"));
        assert!(err.contains("run.sh:2"));
    }

    #[test]
    fn test_cross_node_ref_in_comment_ignored() {
        let temp = TempDir::new().unwrap();
        let job = create_job(
            temp.path(),
            "02-analysis",
            "#!/bin/bash\n# cp ../01-prep/output.csv .\npython main.py\n",
        );
        assert!(check_cross_node_references(&[job]).is_ok());
    }

    #[test]
    fn test_cross_node_ref_multiple_violations() {
        let temp = TempDir::new().unwrap();
        let job1 = create_job(
            temp.path(),
            "02-analysis",
            "#!/bin/bash\ncp ../01-prep/outputs/result.csv .\n",
        );
        let job2 = create_job(
            temp.path(),
            "03-report",
            "#!/bin/bash\ncat ../02-analysis/outputs/summary.txt\n",
        );
        let err = check_cross_node_references(&[job1, job2]).unwrap_err();
        assert!(err.contains("[02-analysis]"));
        assert!(err.contains("[03-report]"));
    }

    #[test]
    fn a_missing_run_script_is_reported() {
        let tmp = TempDir::new().unwrap();
        let present = create_job(tmp.path(), "01-present", "echo hi\n");
        let missing = create_job(tmp.path(), "02-missing", "echo hi\n");
        fs::remove_file(missing.path.join("run.sh")).unwrap();

        let err = check_run_scripts(&[present, missing]).unwrap_err();
        assert!(err.contains("[02-missing] run.sh"), "{err}");
        assert!(!err.contains("01-present"), "{err}");
    }

    #[test]
    fn present_run_scripts_pass() {
        let tmp = TempDir::new().unwrap();
        let job = create_job(tmp.path(), "01-job", "echo hi\n");
        assert!(check_run_scripts(&[job]).is_ok());
    }

    fn workflow_with_version(version: &str) -> job_config::workflow::WorkflowMeta {
        toml::from_str(&format!(
            "name = \"t\"\ndescription = \"t\"\nschema_version = \"{version}\"\n"
        ))
        .unwrap()
    }

    #[test]
    fn schema_version_refuses_a_newer_major_or_a_malformed_value() {
        let err = check_schema_version(&workflow_with_version("2.0")).unwrap_err();
        assert!(err.contains("upgrade silva"), "{err}");
        let err = check_schema_version(&workflow_with_version("v1")).unwrap_err();
        assert!(err.contains("MAJOR.MINOR"), "{err}");
    }

    #[test]
    fn schema_version_notes_a_newer_minor_and_passes_a_known_one() {
        let note = check_schema_version(&workflow_with_version("1.1")).unwrap();
        assert!(note.is_some_and(|n| n.contains("format 1.1")));
        assert_eq!(
            check_schema_version(&workflow_with_version("1.0")),
            Ok(None)
        );
    }
}

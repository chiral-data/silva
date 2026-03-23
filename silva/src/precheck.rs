//! Pre-execution checks for workflow scripts.
//!
//! Validates workflow conventions before any containers are started:
//! - Rejects scripts containing install commands (should be in Docker images)
//! - Requires `input_files/` folder when dependency-free jobs exist
//! - Rejects scripts with cross-node `../` path references

use std::fs;
use std::path::Path;

use crate::components::workflow::JobFolder;

/// A single violation found in a script.
struct Violation {
    job_name: String,
    script_name: String,
    line_number: usize,
    line_content: String,
}

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

/// Checks all job scripts for install commands.
///
/// Returns `Ok(())` if no install commands are found, or `Err(message)`
/// with a formatted error listing all violations.
pub fn check_install_commands(jobs: &[JobFolder]) -> Result<(), String> {
    let mut violations = Vec::new();

    for job in jobs {
        let job_dir = &job.path;

        // Load job meta to find configured script names
        let script_names = match job.load_meta() {
            Ok(meta) => vec![meta.scripts.pre, meta.scripts.run, meta.scripts.post],
            Err(_) => vec![
                "pre_run.sh".to_string(),
                "run.sh".to_string(),
                "post_run.sh".to_string(),
            ],
        };

        for script_name in &script_names {
            let script_path = job_dir.join(script_name);
            if let Some(mut script_violations) = scan_script(&job.name, script_name, &script_path) {
                violations.append(&mut script_violations);
            }
        }
    }

    if violations.is_empty() {
        return Ok(());
    }

    let mut msg = String::from(
        "Install commands found in workflow scripts.\n\
         Dependencies must be pre-baked into Docker images.\n",
    );

    for v in &violations {
        msg.push_str(&format!(
            "\n  [{}] {}:{} — {}",
            v.job_name,
            v.script_name,
            v.line_number,
            v.line_content.trim()
        ));
    }

    msg.push_str("\n\nFix: Move install commands into Dockerfiles and rebuild the images.");

    Err(msg)
}

/// Scans a single script file for install command patterns.
fn scan_script(job_name: &str, script_name: &str, path: &Path) -> Option<Vec<Violation>> {
    let content = fs::read_to_string(path).ok()?;
    let mut violations = Vec::new();

    for (line_idx, line) in content.lines().enumerate() {
        let trimmed = line.trim();

        // Skip comments
        if trimmed.starts_with('#') {
            continue;
        }

        if line_has_install_command(trimmed) {
            violations.push(Violation {
                job_name: job_name.to_string(),
                script_name: script_name.to_string(),
                line_number: line_idx + 1,
                line_content: trimmed.to_string(),
            });
        }
    }

    if violations.is_empty() {
        None
    } else {
        Some(violations)
    }
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

/// Checks all job scripts for cross-node `../` path references.
///
/// Jobs must use their `inputs/` folder instead of relative paths to siblings.
/// Returns `Err` listing all violations if any are found.
pub fn check_cross_node_references(jobs: &[JobFolder]) -> Result<(), String> {
    let mut violations = Vec::new();

    for job in jobs {
        let job_dir = &job.path;

        let script_names = match job.load_meta() {
            Ok(meta) => vec![meta.scripts.pre, meta.scripts.run, meta.scripts.post],
            Err(_) => vec![
                "pre_run.sh".to_string(),
                "run.sh".to_string(),
                "post_run.sh".to_string(),
            ],
        };

        for script_name in &script_names {
            let script_path = job_dir.join(script_name);
            if let Some(mut script_violations) =
                scan_script_for_cross_node_refs(&job.name, script_name, &script_path)
            {
                violations.append(&mut script_violations);
            }
        }
    }

    if violations.is_empty() {
        return Ok(());
    }

    let mut msg = String::from(
        "Cross-node path references found in workflow scripts.\n\
         Jobs must use the inputs/ folder, not relative paths to siblings.\n",
    );

    for v in &violations {
        msg.push_str(&format!(
            "\n  [{}] {}:{} — {}",
            v.job_name,
            v.script_name,
            v.line_number,
            v.line_content.trim()
        ));
    }

    msg.push_str("\n\nFix: Use files from the inputs/ directory instead of ../");

    Err(msg)
}

/// Scans a single script file for `../` path patterns.
fn scan_script_for_cross_node_refs(
    job_name: &str,
    script_name: &str,
    path: &Path,
) -> Option<Vec<Violation>> {
    let content = fs::read_to_string(path).ok()?;
    let mut violations = Vec::new();

    for (line_idx, line) in content.lines().enumerate() {
        let trimmed = line.trim();

        // Skip comments
        if trimmed.starts_with('#') {
            continue;
        }

        if trimmed.contains("../") {
            violations.push(Violation {
                job_name: job_name.to_string(),
                script_name: script_name.to_string(),
                line_number: line_idx + 1,
                line_content: trimmed.to_string(),
            });
        }
    }

    if violations.is_empty() {
        None
    } else {
        Some(violations)
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
}

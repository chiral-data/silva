//! Pre-execution checks for workflow scripts.
//!
//! Scans job scripts for install commands (pip install, apt-get install, etc.)
//! that should be baked into Docker images instead. If any are found, the
//! workflow is rejected before any containers are started.

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
            if let Some(mut script_violations) = scan_script(&job.name, script_name, &script_path)
            {
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
        let job = create_job(
            temp.path(),
            "01-apk",
            "#!/bin/bash\napk  add  git\n",
        );
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
}

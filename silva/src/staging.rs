//! Staging a job's inputs before it runs: dependency outputs and the
//! workflow's `input_files/`, into `<job>/inputs/`.
//!
//! Shared by `silva run` and the TUI. Nothing here prints: each function
//! returns what it would have said, so the TUI's terminal stays clean and
//! each caller decides where a message goes.

use std::path::Path;

use globset::GlobSetBuilder;

use crate::components::workflow::JobFolder;
use crate::utils::copy_dir_recursive;
use job_config::job::JobMeta;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Info,
    Warning,
}

/// One line a staging step would have printed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    pub level: Level,
    /// The job it is about, when it is about one.
    pub job: Option<String>,
    pub text: String,
}

macro_rules! info {
    ($out:expr, $job:expr, $($arg:tt)*) => {
        $out.push(Notice { level: Level::Info, job: Option::<&str>::map($job, String::from), text: format!($($arg)*) })
    };
}

macro_rules! warn {
    ($out:expr, $job:expr, $($arg:tt)*) => {
        $out.push(Notice { level: Level::Warning, job: Option::<&str>::map($job, String::from), text: format!($($arg)*) })
    };
}

/// Copies input files from dependency jobs' outputs to the current job's inputs folder.
pub(crate) fn copy_input_files_from_dependencies(
    workflow_path: &Path,
    current_job: &JobFolder,
    all_jobs: &[JobFolder],
    config: &JobMeta,
    dependencies: &[String],
    notices: &mut Vec<Notice>,
) -> Result<usize, String> {
    let job = Some(current_job.name.as_str());
    use std::collections::HashSet;
    use std::fs;

    if dependencies.is_empty() {
        return Ok(0);
    }

    // Copy to inputs/ subfolder for clear separation
    let inputs_dir = workflow_path.join(&current_job.name).join("inputs");
    fs::create_dir_all(&inputs_dir).map_err(|e| format!("Failed to create inputs dir: {e}"))?;

    let mut copied_files: HashSet<String> = HashSet::new();

    for dep_job_name in dependencies {
        // Find the dependency job
        let dep_job = all_jobs.iter().find(|j| &j.name == dep_job_name);
        if dep_job.is_none() {
            info!(
                notices,
                job, "Warning: Dependency job '{dep_job_name}' not found"
            );
            continue;
        }

        // Look in @complete/ first (job already finished), fall back to original location
        let complete_outputs_dir = workflow_path
            .join("@complete")
            .join(dep_job_name)
            .join("outputs");
        let dep_outputs_dir = if complete_outputs_dir.exists() {
            complete_outputs_dir
        } else {
            workflow_path.join(dep_job_name).join("outputs")
        };
        if !dep_outputs_dir.exists() {
            info!(
                notices,
                job, "No outputs found for dependency '{dep_job_name}', skipping"
            );
            continue;
        }

        // Determine which files to copy
        let files_to_copy: Vec<std::path::PathBuf> = if config.inputs.is_empty() {
            // Copy all files from outputs/
            match fs::read_dir(&dep_outputs_dir) {
                Ok(entries) => entries.filter_map(|e| e.ok()).map(|e| e.path()).collect(),
                Err(e) => {
                    info!(
                        notices,
                        job, "Error reading outputs from '{dep_job_name}': {e}"
                    );
                    continue;
                }
            }
        } else {
            // Copy only matching files based on input patterns
            let mut builder = GlobSetBuilder::new();
            for pattern in &config.inputs {
                match globset::Glob::new(pattern) {
                    Ok(g) => {
                        builder.add(g);
                    }
                    Err(e) => info!(notices, job, "Invalid glob pattern '{pattern}': {e}"),
                }
            }
            match builder.build() {
                Ok(matcher) => match fs::read_dir(&dep_outputs_dir) {
                    Ok(entries) => entries
                        .filter_map(|e| e.ok())
                        .map(|e| e.path())
                        .filter(|p| p.file_name().map(|n| matcher.is_match(n)).unwrap_or(false))
                        .collect(),
                    Err(e) => {
                        info!(
                            notices,
                            job, "Error reading outputs from '{dep_job_name}': {e}"
                        );
                        continue;
                    }
                },
                Err(e) => {
                    info!(
                        notices,
                        job, "Failed to build glob matcher for '{dep_job_name}': {e}"
                    );
                    continue;
                }
            }
        };

        // Copy files to inputs/ directory
        for source_path in files_to_copy {
            if let Some(filename) = source_path.file_name() {
                let filename_str = filename.to_string_lossy().to_string();
                let dest_path = inputs_dir.join(filename);

                // Check for conflicts
                if copied_files.contains(&filename_str) {
                    info!(
                        notices,
                        job,
                        "Warning: File '{filename_str}' already copied, skipping from '{dep_job_name}'"
                    );
                    continue;
                }

                // Copy the file or directory
                if source_path.is_file() {
                    match fs::copy(&source_path, &dest_path) {
                        Ok(_) => {
                            copied_files.insert(filename_str.clone());
                            info!(
                                notices,
                                job, "Copied '{filename_str}' from '{dep_job_name}'"
                            );
                        }
                        Err(e) => {
                            info!(notices, job, "Error copying '{filename_str}': {e}");
                        }
                    }
                } else if source_path.is_dir() {
                    match copy_dir_recursive(&source_path, &dest_path) {
                        Ok(count) => {
                            copied_files.insert(filename_str.clone());
                            info!(
                                notices,
                                job,
                                "Copied directory '{filename_str}/' ({count} files) from '{dep_job_name}'"
                            );
                        }
                        Err(e) => {
                            info!(
                                notices,
                                job, "Error copying directory '{filename_str}': {e}"
                            );
                        }
                    }
                }
            }
        }
    }

    Ok(copied_files.len())
}

/// Copies files from the workflow's `input_files/` folder to all jobs without dependencies.
///
/// If the `input_files/` folder exists, all its contents are copied to each dependency-free
/// job's `inputs/` subfolder. If the folder doesn't exist, a hint is printed.
pub(crate) fn copy_input_files_to_dependency_free_jobs(
    workflow_path: &Path,
    temp_workflow_path: &Path,
    sorted_jobs: &[JobFolder],
    workflow_metadata: &job_config::workflow::WorkflowMeta,
    notices: &mut Vec<Notice>,
) {
    use std::fs;

    let input_files_path = workflow_path.join("input_files");

    if !input_files_path.is_dir() {
        info!(
            notices,
            None, "Hint: No 'input_files' folder found in workflow"
        );
        return;
    }

    // Find all jobs with no dependencies
    let jobs_without_deps: Vec<&JobFolder> = sorted_jobs
        .iter()
        .filter(|job| workflow_metadata.get_job_dependencies(&job.name).is_empty())
        .collect();

    if jobs_without_deps.is_empty() {
        info!(notices, None, "Warning: No jobs to copy input files to");
        return;
    }

    // Copy input_files to each job without dependencies
    for job in &jobs_without_deps {
        // Copy to inputs/ subfolder for clear separation
        let inputs_dir = temp_workflow_path.join(&job.name).join("inputs");
        if let Err(e) = fs::create_dir_all(&inputs_dir) {
            warn!(
                notices,
                Some(job.name.as_str()),
                "Error creating inputs folder for '{}': {e}",
                job.name
            );
            continue;
        }

        // Read and copy all files from input_files/
        let entries = match fs::read_dir(&input_files_path) {
            Ok(entries) => entries,
            Err(e) => {
                warn!(
                    notices,
                    Some(job.name.as_str()),
                    "Error reading input_files folder: {e}"
                );
                return;
            }
        };

        let mut copied_count = 0;
        for entry in entries.flatten() {
            let source = entry.path();
            let dest = inputs_dir.join(entry.file_name());

            let result = if source.is_file() {
                fs::copy(&source, &dest).map(|_| ())
            } else if source.is_dir() {
                copy_dir_recursive(&source, &dest).map(|_| ())
            } else {
                continue;
            };

            match result {
                Ok(()) => copied_count += 1,
                Err(e) => warn!(
                    notices,
                    Some(job.name.as_str()),
                    "Error copying '{}': {e}",
                    entry.file_name().to_string_lossy()
                ),
            }
        }

        if copied_count > 0 {
            info!(
                notices,
                Some(job.name.as_str()),
                "Copied {} item(s) from 'input_files/' to '{}/inputs/'",
                copied_count,
                job.name
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use job_config::job::{Container, Scripts};
    use std::collections::HashMap;
    use std::fs;

    fn make_job_meta(inputs: Vec<&str>) -> JobMeta {
        JobMeta {
            name: "test".to_string(),
            description: String::new(),
            retry: None,
            timeout: None,
            container: Container::new("ubuntu:latest".to_string()),
            scripts: Scripts {
                pre: String::new(),
                run: "run.sh".to_string(),
                post: String::new(),
            },
            inputs: inputs.into_iter().map(String::from).collect(),
            outputs: vec![],
            params: HashMap::new(),
            resources: None,
        }
    }

    fn setup_dep_outputs(workflow: &Path, dep_name: &str, files: &[&str]) {
        let outputs_dir = workflow.join(dep_name).join("outputs");
        fs::create_dir_all(&outputs_dir).unwrap();
        for f in files {
            fs::write(outputs_dir.join(f), f).unwrap();
        }
    }

    fn list_inputs(workflow: &Path, job_name: &str) -> Vec<String> {
        let inputs_dir = workflow.join(job_name).join("inputs");
        let mut names: Vec<String> = fs::read_dir(inputs_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn brace_pattern_copies_matching_extensions() {
        let tmp = tempfile::tempdir().unwrap();
        let wf = tmp.path();
        setup_dep_outputs(
            wf,
            "01-produce",
            &["seq.fasta", "ref.fa", "proteins.faa", "notes.txt"],
        );

        let producer = JobFolder::new("01-produce".to_string(), wf.join("01-produce"));
        let consumer = JobFolder::new("02-consume".to_string(), wf.join("02-consume"));
        let config = make_job_meta(vec!["*.{fasta,fa,faa}"]);

        let n = copy_input_files_from_dependencies(
            wf,
            &consumer,
            &[producer],
            &config,
            &["01-produce".to_string()],
            &mut Vec::new(),
        )
        .unwrap();

        assert_eq!(n, 3);
        assert_eq!(
            list_inputs(wf, "02-consume"),
            vec!["proteins.faa", "ref.fa", "seq.fasta"]
        );
    }

    #[test]
    fn brace_pattern_excludes_non_matching_files() {
        let tmp = tempfile::tempdir().unwrap();
        let wf = tmp.path();
        setup_dep_outputs(wf, "01-produce", &["a.txt", "b.log", "c.fasta"]);

        let producer = JobFolder::new("01-produce".to_string(), wf.join("01-produce"));
        let consumer = JobFolder::new("02-consume".to_string(), wf.join("02-consume"));
        let config = make_job_meta(vec!["*.{fasta,fa,faa}"]);

        let n = copy_input_files_from_dependencies(
            wf,
            &consumer,
            &[producer],
            &config,
            &["01-produce".to_string()],
            &mut Vec::new(),
        )
        .unwrap();

        assert_eq!(n, 1);
        assert_eq!(list_inputs(wf, "02-consume"), vec!["c.fasta"]);
    }

    #[test]
    fn plain_pattern_still_works() {
        let tmp = tempfile::tempdir().unwrap();
        let wf = tmp.path();
        setup_dep_outputs(
            wf,
            "01-produce",
            &["data.csv", "metadata.json", "other.txt"],
        );

        let producer = JobFolder::new("01-produce".to_string(), wf.join("01-produce"));
        let consumer = JobFolder::new("02-consume".to_string(), wf.join("02-consume"));
        let config = make_job_meta(vec!["*.csv", "*.json"]);

        let n = copy_input_files_from_dependencies(
            wf,
            &consumer,
            &[producer],
            &config,
            &["01-produce".to_string()],
            &mut Vec::new(),
        )
        .unwrap();

        assert_eq!(n, 2);
        assert_eq!(
            list_inputs(wf, "02-consume"),
            vec!["data.csv", "metadata.json"]
        );
    }

    #[test]
    fn empty_inputs_copies_all_files() {
        let tmp = tempfile::tempdir().unwrap();
        let wf = tmp.path();
        setup_dep_outputs(wf, "01-produce", &["a.txt", "b.csv", "c.fasta"]);

        let producer = JobFolder::new("01-produce".to_string(), wf.join("01-produce"));
        let consumer = JobFolder::new("02-consume".to_string(), wf.join("02-consume"));
        let config = make_job_meta(vec![]);

        let n = copy_input_files_from_dependencies(
            wf,
            &consumer,
            &[producer],
            &config,
            &["01-produce".to_string()],
            &mut Vec::new(),
        )
        .unwrap();

        assert_eq!(n, 3);
        assert_eq!(
            list_inputs(wf, "02-consume"),
            vec!["a.txt", "b.csv", "c.fasta"]
        );
    }

    fn workflow_meta(dependencies: &str) -> job_config::workflow::WorkflowMeta {
        toml::from_str(&format!(
            "name = \"w\"\ndescription = \"d\"\n{dependencies}"
        ))
        .unwrap()
    }

    #[test]
    fn input_files_land_in_inputs_of_dependency_free_jobs_only() {
        let source = tempfile::tempdir().unwrap();
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir_all(source.path().join("input_files")).unwrap();
        fs::write(source.path().join("input_files/seed.csv"), "x").unwrap();
        let jobs: Vec<JobFolder> = ["01-first", "02-second"]
            .iter()
            .map(|n| JobFolder::new(n.to_string(), temp.path().join(n)))
            .collect();
        let meta = workflow_meta("[dependencies]\n02-second = [\"01-first\"]\n");

        let mut notices = Vec::new();
        copy_input_files_to_dependency_free_jobs(
            source.path(),
            temp.path(),
            &jobs,
            &meta,
            &mut notices,
        );

        assert_eq!(list_inputs(temp.path(), "01-first"), vec!["seed.csv"]);
        assert!(!temp.path().join("02-second/inputs").exists());
        assert_eq!(
            notices,
            vec![Notice {
                level: Level::Info,
                job: Some("01-first".to_string()),
                text: "Copied 1 item(s) from 'input_files/' to '01-first/inputs/'".to_string(),
            }]
        );
    }

    #[test]
    fn notices_name_the_job_they_are_about() {
        let tmp = tempfile::tempdir().unwrap();
        let wf = tmp.path();
        setup_dep_outputs(wf, "01-produce", &["a.txt"]);
        let producer = JobFolder::new("01-produce".to_string(), wf.join("01-produce"));
        let consumer = JobFolder::new("02-consume".to_string(), wf.join("02-consume"));

        let mut notices = Vec::new();
        copy_input_files_from_dependencies(
            wf,
            &consumer,
            &[producer],
            &make_job_meta(vec![]),
            &["01-produce".to_string()],
            &mut notices,
        )
        .unwrap();
        assert_eq!(notices.len(), 1);
        assert_eq!(notices[0].job.as_deref(), Some("02-consume"));
        assert_eq!(notices[0].text, "Copied 'a.txt' from '01-produce'");

        // No input_files/ at all: a hint that belongs to no job.
        let mut notices = Vec::new();
        copy_input_files_to_dependency_free_jobs(wf, wf, &[], &workflow_meta(""), &mut notices);
        assert_eq!(notices[0].job, None);
        assert!(notices[0].text.starts_with("Hint:"));
    }
}

//! Headless workflow execution mode.
//!
//! This module provides functionality to run workflows without the TUI,
//! outputting logs directly to stdout/stderr.

use std::collections::HashMap;
use std::path::Path;
use std::time::SystemTime;

use tempfile::TempDir;
use tokio::sync::mpsc;

use crate::components::docker::{
    executor::DockerExecutor,
    job::JobStatus,
    logs::{LogLine, LogSource},
};
use crate::components::workflow::{JobFolder, JobScanner, WorkflowFolder};
use job_config::job::JobMeta;

/// Runs a workflow in headless mode, outputting logs to stdout/stderr.
///
/// # Arguments
///
/// * `workflow_path` - Path to the workflow directory
///
/// # Returns
///
/// * `Ok(())` - Workflow completed successfully
/// * `Err(String)` - Error message if workflow failed
pub async fn run_workflow(workflow_path: &Path) -> Result<(), String> {
    // Validate workflow path
    let workflow_path = workflow_path
        .canonicalize()
        .map_err(|e| format!("Invalid workflow path: {e}"))?;

    if !workflow_path.is_dir() {
        return Err(format!(
            "Workflow path is not a directory: {}",
            workflow_path.display()
        ));
    }

    // Create WorkflowFolder
    let workflow_name = workflow_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("workflow")
        .to_string();

    let workflow_folder = WorkflowFolder::new(
        workflow_name.clone(),
        workflow_path.clone(),
        Some(SystemTime::now()),
    );

    // Create temp workflow folder
    let temp_workflow_dir = create_temp_workflow_folder(&workflow_path)
        .map_err(|e| format!("Failed to create temp workflow: {e}"))?;
    let temp_workflow_path = temp_workflow_dir.path().to_path_buf();

    println!("Running workflow: {workflow_name}");
    println!("Temp folder: {}", temp_workflow_path.display());

    // Scan for jobs in temp folder
    let jobs = JobScanner::scan_jobs(&temp_workflow_path)
        .map_err(|e| format!("Failed to scan jobs: {e}"))?;

    if jobs.is_empty() {
        return Err("No jobs found in workflow".to_string());
    }

    println!("Found {} job(s)", jobs.len());

    // Load workflow metadata
    let workflow_metadata = workflow_folder
        .load_workflow_metadata()
        .ok()
        .flatten()
        .unwrap_or_else(|| {
            job_config::workflow::WorkflowMeta::new(workflow_name.clone(), String::new())
        });

    // Load workflow parameters
    let workflow_params = workflow_folder
        .load_workflow_params()
        .ok()
        .flatten()
        .unwrap_or_default();

    if !workflow_params.is_empty() {
        println!(
            "Loaded {} global workflow parameter(s)",
            workflow_params.len()
        );
    }

    // Sort jobs in dependency order
    let sorted_jobs = topological_sort_jobs(&jobs, &workflow_metadata)?;

    println!(
        "Execution order: {}",
        sorted_jobs
            .iter()
            .map(|j| j.name.as_str())
            .collect::<Vec<_>>()
            .join(" -> ")
    );
    println!();

    // Create message channel for logs
    let (tx, mut rx) = mpsc::channel::<(usize, JobStatus, LogLine)>(32);
    let (_cancel_tx, mut cancel_rx) = mpsc::channel::<()>(1);

    // Create job name to index map
    let job_name_to_idx: HashMap<String, usize> = jobs
        .iter()
        .enumerate()
        .map(|(idx, job)| (job.name.clone(), idx))
        .collect();

    let jobs_len = jobs.len();
    let sorted_jobs_clone = sorted_jobs.clone();
    let workflow_folder_clone = workflow_folder.clone();
    let temp_workflow_path_clone = temp_workflow_path.clone();

    // Spawn workflow execution task
    let exec_handle = tokio::spawn(async move {
        let mut docker_executor = match DockerExecutor::new(tx.clone()) {
            Ok(executor) => executor,
            Err(e) => {
                let log_line = LogLine::new(
                    LogSource::Stderr,
                    format!("Failed to create Docker executor: {e}"),
                );
                let _ = tx.send((0, JobStatus::Failed, log_line)).await;
                let _ = tx
                    .send((jobs_len, JobStatus::Failed, LogLine::empty()))
                    .await;
                return Err(format!("Docker initialization failed: {e}"));
            }
        };

        let mut container_registry: HashMap<String, String> = HashMap::new();
        let mut workflow_failed = false;

        for job in sorted_jobs_clone.iter() {
            let idx = *job_name_to_idx.get(&job.name).unwrap();

            match job.load_meta() {
                Ok(config) => {
                    docker_executor.set_job_idx(idx);

                    let job_params = job.load_params().ok().flatten().unwrap_or_default();

                    // Copy input files from dependencies before running
                    let job_deps = workflow_metadata.get_job_dependencies(&job.name);
                    if let Err(e) = copy_input_files_from_dependencies(
                        &temp_workflow_path_clone,
                        job,
                        &sorted_jobs_clone,
                        &config,
                        job_deps,
                    ) {
                        let log_line = LogLine::new(
                            LogSource::Stderr,
                            format!("Warning: Failed to copy input files: {e}"),
                        );
                        let _ = tx.send((idx, JobStatus::Running, log_line)).await;
                    }

                    match docker_executor
                        .run_job(
                            (
                                &workflow_folder_clone.name,
                                &temp_workflow_path_clone,
                                &workflow_params,
                            ),
                            (job, &config, &job_params),
                            &mut container_registry,
                            &mut cancel_rx,
                        )
                        .await
                    {
                        Ok(_container_id) => {}
                        Err(e) => {
                            let log_line = LogLine::new(
                                LogSource::Stderr,
                                format!("Job '{}' failed: {e}", job.name),
                            );
                            let _ = tx.send((idx, JobStatus::Failed, log_line)).await;
                            workflow_failed = true;
                            break;
                        }
                    }
                }
                Err(e) => {
                    let log_line = LogLine::new(
                        LogSource::Stderr,
                        format!("Failed to load config for '{}': {e}", job.name),
                    );
                    let _ = tx.send((idx, JobStatus::Failed, log_line)).await;
                    workflow_failed = true;
                    break;
                }
            }
        }

        // Cleanup containers
        let container_ids: Vec<String> = container_registry.values().cloned().collect();
        docker_executor.cleanup_containers(&container_ids).await;

        let final_status = if workflow_failed {
            JobStatus::Failed
        } else {
            JobStatus::Completed
        };
        let _ = tx.send((jobs_len, final_status, LogLine::empty())).await;

        if workflow_failed {
            Err("Workflow failed".to_string())
        } else {
            Ok(())
        }
    });

    // Process log messages and output to stdout/stderr
    let mut current_job: Option<String> = None;
    let mut workflow_result = Ok(());

    while let Some((idx, status, log_line)) = rx.recv().await {
        // Check if workflow is complete
        if idx == jobs.len() {
            if status == JobStatus::Failed {
                workflow_result = Err("Workflow failed".to_string());
            }
            break;
        }

        // Get job name
        let job_name = jobs.get(idx).map(|j| j.name.as_str()).unwrap_or("unknown");

        // Print job header when switching jobs
        if current_job.as_deref() != Some(job_name) {
            if current_job.is_some() {
                println!();
            }
            println!("=== Job: {job_name} ===");
            current_job = Some(job_name.to_string());
        }

        // Print log line
        if !log_line.content.is_empty() {
            match log_line.source {
                LogSource::Stdout => println!("{}", log_line.content),
                LogSource::Stderr => eprintln!("{}", log_line.content),
            }
        }

        // Print status changes
        if status == JobStatus::Completed {
            println!("[{job_name}] Completed");
        } else if status == JobStatus::Failed {
            eprintln!("[{job_name}] Failed");
            workflow_result = Err(format!("Job '{job_name}' failed"));
        }
    }

    // Wait for execution to finish
    let _ = exec_handle.await;

    // Keep the temp folder for user inspection
    let temp_path = temp_workflow_dir.keep();

    println!();
    match &workflow_result {
        Ok(()) => {
            println!("Workflow completed successfully");
            println!();
            println!("Output folder: {}", temp_path.display());
            println!("  (This folder will persist until you delete it manually)");
        }
        Err(e) => {
            eprintln!("Workflow failed: {e}");
            println!();
            println!("Working folder: {}", temp_path.display());
            println!("  (You can inspect this folder to debug the issue)");
        }
    }

    workflow_result
}

/// Performs topological sort on jobs based on their dependencies.
fn topological_sort_jobs(
    jobs: &[JobFolder],
    workflow_metadata: &job_config::workflow::WorkflowMeta,
) -> Result<Vec<JobFolder>, String> {
    use std::collections::VecDeque;

    if jobs.is_empty() {
        return Ok(Vec::new());
    }

    // Build a map of job names to job data
    let job_map: HashMap<String, JobFolder> =
        jobs.iter().map(|j| (j.name.clone(), j.clone())).collect();

    // Build dependency graph
    let mut dependents: HashMap<String, Vec<String>> = HashMap::new();
    let mut in_degree: HashMap<String, usize> = HashMap::new();

    // Initialize
    for job in jobs {
        in_degree.insert(job.name.clone(), 0);
        dependents.entry(job.name.clone()).or_default();
    }

    // Build the graph
    for job in jobs {
        let job_deps = workflow_metadata.get_job_dependencies(&job.name);

        for dep_name in job_deps {
            if !job_map.contains_key(dep_name) {
                return Err(format!(
                    "Job '{}' depends on '{}', but '{}' does not exist",
                    job.name, dep_name, dep_name
                ));
            }

            dependents
                .entry(dep_name.clone())
                .or_default()
                .push(job.name.clone());

            *in_degree.get_mut(&job.name).unwrap() += 1;
        }
    }

    // Kahn's algorithm
    let mut queue: VecDeque<String> = in_degree
        .iter()
        .filter(|&(_, &degree)| degree == 0)
        .map(|(name, _)| name.clone())
        .collect();

    let mut sorted_jobs = Vec::new();

    while let Some(job_name) = queue.pop_front() {
        if let Some(job) = job_map.get(&job_name) {
            sorted_jobs.push(job.clone());
        }

        if let Some(deps) = dependents.get(&job_name) {
            for dependent in deps {
                let degree = in_degree.get_mut(dependent).unwrap();
                *degree -= 1;
                if *degree == 0 {
                    queue.push_back(dependent.clone());
                }
            }
        }
    }

    if sorted_jobs.len() != jobs.len() {
        let processed: std::collections::HashSet<_> = sorted_jobs.iter().map(|j| &j.name).collect();
        let unprocessed: Vec<_> = jobs
            .iter()
            .filter(|j| !processed.contains(&j.name))
            .map(|j| j.name.as_str())
            .collect();

        return Err(format!(
            "Circular dependency detected: {}",
            unprocessed.join(", ")
        ));
    }

    Ok(sorted_jobs)
}

/// Copies input files from dependency jobs' outputs to the current job folder.
fn copy_input_files_from_dependencies(
    workflow_path: &Path,
    current_job: &JobFolder,
    all_jobs: &[JobFolder],
    config: &JobMeta,
    dependencies: &[String],
) -> Result<usize, String> {
    use std::collections::HashSet;
    use std::fs;

    if dependencies.is_empty() {
        return Ok(0);
    }

    let current_job_dir = workflow_path.join(&current_job.name);
    let mut copied_files: HashSet<String> = HashSet::new();

    for dep_job_name in dependencies {
        // Find the dependency job
        let dep_job = all_jobs.iter().find(|j| &j.name == dep_job_name);
        if dep_job.is_none() {
            println!("Warning: Dependency job '{dep_job_name}' not found");
            continue;
        }

        let dep_outputs_dir = workflow_path.join(dep_job_name).join("outputs");
        if !dep_outputs_dir.exists() {
            println!("No outputs found for dependency '{dep_job_name}', skipping");
            continue;
        }

        // Determine which files to copy
        let files_to_copy: Vec<std::path::PathBuf> = if config.inputs.is_empty() {
            // Copy all files from outputs/
            match fs::read_dir(&dep_outputs_dir) {
                Ok(entries) => entries.filter_map(|e| e.ok()).map(|e| e.path()).collect(),
                Err(e) => {
                    println!("Error reading outputs from '{dep_job_name}': {e}");
                    continue;
                }
            }
        } else {
            // Copy only matching files based on input patterns
            let mut matching_files = Vec::new();
            for pattern in &config.inputs {
                let glob_pattern = dep_outputs_dir.join(pattern).to_string_lossy().to_string();
                match glob::glob(&glob_pattern) {
                    Ok(paths) => {
                        for path in paths.flatten() {
                            matching_files.push(path);
                        }
                    }
                    Err(e) => {
                        println!("Invalid glob pattern '{pattern}': {e}");
                    }
                }
            }
            matching_files
        };

        // Copy files to current job directory
        for source_path in files_to_copy {
            if let Some(filename) = source_path.file_name() {
                let filename_str = filename.to_string_lossy().to_string();
                let dest_path = current_job_dir.join(filename);

                // Check for conflicts
                if copied_files.contains(&filename_str) {
                    println!(
                        "Warning: File '{filename_str}' already copied, skipping from '{dep_job_name}'"
                    );
                    continue;
                }

                // Copy the file or directory
                if source_path.is_file() {
                    match fs::copy(&source_path, &dest_path) {
                        Ok(_) => {
                            copied_files.insert(filename_str.clone());
                            println!("Copied '{filename_str}' from '{dep_job_name}'");
                        }
                        Err(e) => {
                            println!("Error copying '{filename_str}': {e}");
                        }
                    }
                } else if source_path.is_dir() {
                    match copy_dir_recursive(&source_path, &dest_path) {
                        Ok(count) => {
                            copied_files.insert(filename_str.clone());
                            println!(
                                "Copied directory '{filename_str}/' ({count} files) from '{dep_job_name}'"
                            );
                        }
                        Err(e) => {
                            println!("Error copying directory '{filename_str}': {e}");
                        }
                    }
                }
            }
        }
    }

    Ok(copied_files.len())
}

/// Recursively copies a directory and its contents.
fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<usize> {
    use std::fs;

    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }

    let mut file_count = 0;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let dest_path = dst.join(entry.file_name());

        if path.is_dir() {
            file_count += copy_dir_recursive(&path, &dest_path)?;
        } else {
            fs::copy(&path, &dest_path)?;
            file_count += 1;
        }
    }

    Ok(file_count)
}

/// Creates a temporary folder and copies the workflow contents to it.
fn create_temp_workflow_folder(source_path: &Path) -> std::io::Result<TempDir> {
    let now = chrono::Local::now();
    let timestamp = now.format("%Y-%m-%d-%H-%M-%S").to_string();
    let prefix = format!("silva-{timestamp}-");
    let temp_dir = tempfile::Builder::new().prefix(&prefix).tempdir()?;

    // Copy source folder contents to temp directory
    let mut options = fs_extra::dir::CopyOptions::new();
    options.overwrite = true;
    options.copy_inside = true;
    options.content_only = true;

    fs_extra::dir::copy(source_path, temp_dir.path(), &options)
        .map_err(|e| std::io::Error::other(format!("copy folder error {e}")))?;

    Ok(temp_dir)
}

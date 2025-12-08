//! Headless workflow execution mode.
//!
//! This module provides functionality to run workflows without the TUI,
//! outputting logs directly to stdout/stderr.

use std::collections::HashMap;
use std::path::Path;
use std::time::SystemTime;

use tokio::sync::mpsc;

use crate::components::docker::{
    executor::DockerExecutor,
    job::JobStatus,
    logs::{LogLine, LogSource},
};
use crate::components::workflow::{JobFolder, JobScanner, WorkflowFolder};

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

    // Scan for jobs
    let jobs = JobScanner::scan_jobs(&workflow_path)
        .map_err(|e| format!("Failed to scan jobs: {e}"))?;

    if jobs.is_empty() {
        return Err("No jobs found in workflow".to_string());
    }

    println!("Running workflow: {workflow_name}");
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

                    match docker_executor
                        .run_job(
                            &workflow_folder_clone.name,
                            &workflow_folder_clone.path,
                            job,
                            &config,
                            &workflow_params,
                            &job_params,
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
        let _ = tx
            .send((jobs_len, final_status, LogLine::empty()))
            .await;

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

    println!();
    match &workflow_result {
        Ok(()) => println!("Workflow completed successfully"),
        Err(e) => eprintln!("Workflow failed: {e}"),
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
        let processed: std::collections::HashSet<_> =
            sorted_jobs.iter().map(|j| &j.name).collect();
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

//! Headless workflow execution mode.
//!
//! This module provides functionality to run workflows without the TUI,
//! outputting logs directly to stdout/stderr.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::SystemTime;

use tempfile::TempDir;
use tokio::sync::mpsc;

use crate::components::docker::{
    executor::DockerExecutor,
    job::JobStatus,
    logs::{LogLine, LogSource},
};
use crate::components::workflow::{JobFolder, JobScanner, WorkflowFolder};
use crate::events::{Emitter, OutputFormat};

/// Runs a workflow in headless mode, outputting logs to stdout/stderr.
///
/// # Arguments
///
/// * `workflow_path` - Path to the workflow directory
/// * `cli_env_vars` - `KEY=VALUE` strings from `-e/--env`, injected unprefixed into
///   every job's container exec environment, independent of `env_passthrough`
/// * `output` - human text, or newline-delimited JSON events for a caller
///   driving the run programmatically
///
/// # Returns
///
/// * `Ok(())` - Workflow completed successfully
/// * `Err(String)` - Error message if workflow failed
pub async fn run_workflow(
    workflow_path: &Path,
    cli_env_vars: &[String],
    output: OutputFormat,
) -> Result<(), String> {
    let mut emitter = Emitter::new(output);
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

    let docker_socket =
        std::env::var("DOCKER_HOST").unwrap_or_else(|_| "unix:///var/run/docker.sock".to_string());
    emitter.preamble(&docker_socket, &workflow_name, &temp_workflow_path);

    // Scan for jobs in temp folder
    let jobs = JobScanner::scan_jobs(&temp_workflow_path)
        .map_err(|e| format!("Failed to scan jobs: {e}"))?;

    if jobs.is_empty() {
        return Err("No jobs found in workflow".to_string());
    }

    emitter.jobs_found(jobs.len());

    // Load workflow metadata (dependencies are managed here, not in job.toml).
    // A missing file means "no dependencies"; a broken one must not, or the
    // jobs would run in the wrong order.
    let workflow_metadata = workflow_folder
        .load_workflow_metadata()
        .map_err(|e| format!("Cannot read .chiral/workflow.toml: {e}"))?
        .unwrap_or_else(|| {
            job_config::workflow::WorkflowMeta::new(workflow_name.clone(), String::new())
        });

    // Before the sort and prechecks: a newer major may change what they mean.
    if let Some(note) = crate::precheck::check_schema_version(&workflow_metadata)? {
        crate::note!("Note: {note}");
    }

    // Raw values only: run_job fills in the declared defaults. A broken file
    // must not silently become "use the defaults".
    let workflow_params = workflow_folder
        .load_workflow_params()
        .map_err(|e| format!("Cannot read global_params.json: {e}"))?
        .unwrap_or_default();

    emitter.global_params_loaded(workflow_params.len());

    // Sort jobs in dependency order
    let sorted_jobs = topological_sort_jobs(&jobs, &workflow_metadata)?;

    let execution_order: Vec<String> = sorted_jobs.iter().map(|j| j.name.clone()).collect();
    emitter.execution_order(&execution_order);

    // Pre-checks: reject workflows that violate conventions
    crate::precheck::check_install_commands(&sorted_jobs)?;
    crate::precheck::check_cross_node_references(&sorted_jobs)?;
    crate::precheck::check_run_scripts(&sorted_jobs)?;
    crate::precheck::check_input_files_folder(&workflow_path, &sorted_jobs, &workflow_metadata)?;

    // Copy input_files to all jobs without dependencies
    let mut notices = Vec::new();
    crate::staging::copy_input_files_to_dependency_free_jobs(
        &workflow_path,
        &temp_workflow_path,
        &sorted_jobs,
        &workflow_metadata,
        &mut notices,
    );
    print_notices(notices);

    // The started event waits until prechecks have passed and inputs are
    // staged, so a workflow rejected before this point never reports itself as
    // started.
    emitter.workflow_started(workflow_metadata.name.as_str(), &execution_order);

    // Create message channel for logs
    let (tx, mut rx) = mpsc::channel::<(usize, JobStatus, LogLine)>(32);
    let (cancel_tx, mut cancel_rx) = mpsc::channel::<()>(1);

    // Headless runs have no other cancellation path, so Ctrl-C is wired
    // straight to the sender `run_job`/`exec_script` already know how to
    // honour. A second Ctrl-C is a hard exit: if cleanup itself gets stuck on
    // a container that will not stop, the user needs a way out that doesn't
    // require another terminal.
    let cancelled = Arc::new(AtomicBool::new(false));
    let cancelled_for_signal = Arc::clone(&cancelled);
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_err() {
            return;
        }
        cancelled_for_signal.store(true, Ordering::SeqCst);
        let _ = cancel_tx.send(()).await;

        if tokio::signal::ctrl_c().await.is_ok() {
            std::process::exit(130);
        }
    });

    // Create job name to index map
    let job_name_to_idx: HashMap<String, usize> = jobs
        .iter()
        .enumerate()
        .map(|(idx, job)| (job.name.clone(), idx))
        .collect();

    // The reporting side needs these after the executor task takes ownership.
    let job_index_by_name = job_name_to_idx.clone();
    let jobs_len = jobs.len();
    let sorted_jobs_clone = sorted_jobs.clone();
    let temp_workflow_path_clone = temp_workflow_path.clone();
    let cli_env_vars = cli_env_vars.to_vec();
    let source_workflow_path = workflow_path.clone();

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
        docker_executor.detect_host_gpu().await;
        if let Err(e) =
            crate::precheck::check_gpu_required(&sorted_jobs_clone, docker_executor.host_has_gpu())
        {
            let log_line = LogLine::new(LogSource::Stderr, e.clone());
            let _ = tx.send((0, JobStatus::Failed, log_line)).await;
            let _ = tx
                .send((jobs_len, JobStatus::Failed, LogLine::empty()))
                .await;
            return Err(e);
        }

        // Build the local app images this workflow ships in apps/, if any.
        // Their tags exist in no registry, so pulling cannot satisfy them.
        // The build context is the source folder, not the temp copy, so a
        // .dockerignore or symlink behaves as authored.
        let local_apps =
            crate::components::workflow::apps_to_build(&source_workflow_path, &sorted_jobs_clone);
        if let Err(e) = docker_executor.ensure_local_app_images(&local_apps).await {
            let log_line = LogLine::new(
                LogSource::Stderr,
                format!("Local app image build failed: {e}"),
            );
            let _ = tx.send((0, JobStatus::Failed, log_line)).await;
            let _ = tx
                .send((jobs_len, JobStatus::Failed, LogLine::empty()))
                .await;
            return Err(format!("Local app image build failed: {e}"));
        }

        let mut container_registry: HashMap<String, String> = HashMap::new();
        let mut workflow_failed = false;

        for job in sorted_jobs_clone.iter() {
            let idx = *job_name_to_idx.get(&job.name).unwrap();

            match job.load_meta() {
                Ok(config) => {
                    docker_executor.set_job_idx(idx);

                    let job_params = match job.load_params() {
                        Ok(params) => params.unwrap_or_default(),
                        Err(e) => {
                            let log_line = LogLine::new(
                                LogSource::Stderr,
                                format!("Cannot read {}/params.json: {e}", job.name),
                            );
                            let _ = tx.send((idx, JobStatus::Failed, log_line)).await;
                            workflow_failed = true;
                            break;
                        }
                    };

                    // Copy input files from dependencies before running
                    let job_deps = workflow_metadata.get_job_dependencies(&job.name);
                    let mut notices = Vec::new();
                    let copied = crate::staging::copy_input_files_from_dependencies(
                        &temp_workflow_path_clone,
                        job,
                        &sorted_jobs_clone,
                        &config,
                        job_deps,
                        &mut notices,
                    );
                    print_notices(notices);
                    if let Err(e) = copied {
                        let log_line = LogLine::new(
                            LogSource::Stderr,
                            format!("Warning: Failed to copy input files: {e}"),
                        );
                        let _ = tx.send((idx, JobStatus::Running, log_line)).await;
                    }

                    // If this job runs with RUN_MODE=use_dok, prepare and inject the
                    // presigned bundle URLs run_dok.sh needs (script dir + the inputs/
                    // dir copy_input_files_from_dependencies just populated above).
                    let mut job_env_vars = cli_env_vars.clone();
                    let run_mode = crate::infra::dok::resolve_run_mode(
                        &cli_env_vars,
                        workflow_metadata.env_passthrough.as_deref().unwrap_or(&[]),
                    );
                    if run_mode.as_deref() == Some("use_dok") {
                        match crate::infra::dok::prepare_bundle_env_vars(&job.path).await {
                            Ok(extra) => job_env_vars.extend(extra),
                            Err(e) => {
                                let log_line = LogLine::new(
                                    LogSource::Stderr,
                                    format!("DOK bundle preparation failed: {e}"),
                                );
                                let _ = tx.send((idx, JobStatus::Failed, log_line)).await;
                                workflow_failed = true;
                                break;
                            }
                        }
                    }

                    match docker_executor
                        .run_job(
                            (
                                &workflow_metadata,
                                &temp_workflow_path_clone,
                                &workflow_params,
                            ),
                            (job, &config, &job_params),
                            &job_env_vars,
                            &mut container_registry,
                            &mut cancel_rx,
                        )
                        .await
                    {
                        Ok(_container_id) => {
                            // Move completed job to @complete/ to prevent cross-node path access
                            if let Err(e) =
                                move_job_to_complete(&temp_workflow_path_clone, &job.name)
                            {
                                let log_line = LogLine::new(
                                    LogSource::Stderr,
                                    format!(
                                        "Warning: Failed to move '{}' to @complete: {e}",
                                        job.name
                                    ),
                                );
                                let _ = tx.send((idx, JobStatus::Running, log_line)).await;
                            }
                        }
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

    // Process messages and produce output
    let mut workflow_result = Ok(());
    // The job currently being reported, and whether it has already failed —
    // a job's terminal state is decided by the run moving on, not by any one
    // message, because the executor reports `Completed` once per script.
    let mut open_job: Option<(usize, String)> = None;
    let mut open_job_failed: Option<String> = None;
    let mut finished: Vec<String> = Vec::new();

    while let Some((idx, status, log_line)) = rx.recv().await {
        // Check if workflow is complete
        if idx == jobs.len() {
            if status == JobStatus::Failed {
                workflow_result = Err("Workflow failed".to_string());
            }
            break;
        }

        // Get job name
        let job_name = jobs
            .get(idx)
            .map(|j| j.name.clone())
            .unwrap_or_else(|| "unknown".to_string());

        // The run has moved on: close the previous job before reporting this one.
        if let Some((open_idx, open_name)) = open_job.clone()
            && open_idx != idx
        {
            emitter.job_finished(
                &open_name,
                open_idx,
                open_job_failed.is_some(),
                open_job_failed.as_deref(),
            );
            finished.push(open_name);
            open_job_failed = None;
        }
        open_job = Some((idx, job_name.clone()));

        emitter.job_phase(&job_name, idx, &status, log_line.timestamp);

        if !log_line.content.is_empty() {
            emitter.log(
                &job_name,
                idx,
                log_line.source,
                &log_line.content,
                log_line.timestamp,
            );
        }

        // Cleanup lines after a failure arrive as `Completed`; the job still failed.
        if !(status == JobStatus::Completed && open_job_failed.is_some()) {
            emitter.job_status_line(&job_name, &status);
        }

        if status == JobStatus::Failed {
            // Keep silva's own message as the reason, so a caller never has to
            // read it back out of the log stream.
            let reason = if log_line.content.is_empty() {
                format!("Job '{job_name}' failed")
            } else {
                log_line.content.clone()
            };
            open_job_failed = Some(reason);
            workflow_result = Err(format!("Job '{job_name}' failed"));
        }
    }

    // Close the last job the run reached.
    if let Some((open_idx, open_name)) = open_job {
        emitter.job_finished(
            &open_name,
            open_idx,
            open_job_failed.is_some(),
            open_job_failed.as_deref(),
        );
        finished.push(open_name);
    }

    // Anything left in the execution order never ran. Reported explicitly:
    // absence cannot be told apart from "not part of this workflow".
    if workflow_result.is_err() {
        for name in execution_order.iter().filter(|n| !finished.contains(n)) {
            let idx = job_index_by_name.get(name).copied().unwrap_or(0);
            emitter.job_skipped(name, idx, "an earlier job failed");
        }
    }

    // Wait for execution to finish
    let _ = exec_handle.await;

    // Keep the temp folder for user inspection
    let temp_path = temp_workflow_dir.keep();

    // Ctrl-C unwinds through the same job loop a failure does, which already
    // ends in `cleanup_containers` — but the run should report itself as
    // cancelled rather than merely failed.
    if cancelled.load(Ordering::SeqCst) {
        emitter.workflow_cancelled(&temp_path);
        return Err("Workflow cancelled".to_string());
    }

    emitter.workflow_finished(&workflow_result, &temp_path);

    workflow_result
}

/// Performs topological sort on jobs based on their dependencies.
pub(crate) fn topological_sort_jobs(
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

    // Kahn's algorithm, seeded in scan (folder-name) order: seeding from the
    // map would make the order of independent jobs vary from run to run.
    let mut queue: VecDeque<String> = jobs
        .iter()
        .filter(|j| in_degree[&j.name] == 0)
        .map(|j| j.name.clone())
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

/// Prints staging notices as the staging code used to print them itself.
fn print_notices(notices: Vec<crate::staging::Notice>) {
    for notice in notices {
        match notice.level {
            crate::staging::Level::Info => crate::note!("{}", notice.text),
            crate::staging::Level::Warning => crate::warn_note!("{}", notice.text),
        }
    }
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

/// Moves a completed job folder to `@complete/` to prevent cross-node path access.
///
/// After a job finishes, its folder is moved from the temp workflow root into
/// `@complete/{job_name}/`. This ensures that subsequent jobs cannot use relative
/// paths (e.g., `../01-data-prep/`) to access sibling node folders directly,
/// forcing them to use the proper `inputs/` contract.
fn move_job_to_complete(workflow_path: &Path, job_name: &str) -> Result<(), String> {
    use std::fs;

    let source = workflow_path.join(job_name);
    let complete_dir = workflow_path.join("@complete");
    let dest = complete_dir.join(job_name);

    fs::create_dir_all(&complete_dir)
        .map_err(|e| format!("Failed to create @complete directory: {e}"))?;

    fs::rename(&source, &dest)
        .map_err(|e| format!("Failed to move '{job_name}' to @complete: {e}"))?;

    crate::note!("[{job_name}] Moved to @complete/");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn independent_jobs_sort_in_folder_name_order_every_time() {
        // Seeding from the in-degree map made this order vary between runs.
        let names = [
            "01-a", "02-b", "03-c", "04-d", "05-e", "06-f", "07-g", "08-h",
        ];
        let jobs: Vec<JobFolder> = names
            .iter()
            .map(|n| JobFolder::new(n.to_string(), std::path::PathBuf::from(n)))
            .collect();
        let mut meta = job_config::workflow::WorkflowMeta::new("w".into(), String::new());
        meta.dependencies.insert("08-h".into(), vec!["01-a".into()]);

        for _ in 0..20 {
            let order: Vec<String> = topological_sort_jobs(&jobs, &meta)
                .unwrap()
                .into_iter()
                .map(|j| j.name)
                .collect();
            assert_eq!(order, names);
        }
    }
}

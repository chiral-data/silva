use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, Mutex},
};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tempfile::TempDir;
use tokio::sync::mpsc;

use crate::components::workflow::{self, Job};

use super::{
    executor::DockerExecutor,
    job::{JobEntry, JobStatus},
    logs::{LogLine, LogSource},
};

#[derive(Debug)]
pub struct State {
    pub jobs: Vec<Job>,
    pub job_entries: Vec<JobEntry>,
    pub selected_job_index: Option<usize>,
    pub scroll_offset: usize,
    pub rx: Option<mpsc::Receiver<(usize, JobStatus, LogLine)>>,
    pub cancel_tx: Option<mpsc::Sender<()>>,
    pub is_executing_workflow: bool,
    pub workflow_temp_dirs: Arc<Mutex<HashMap<String, Vec<TempDir>>>>,
    pub auto_scroll_enabled: bool,
    pub last_viewport_width: usize,
    pub last_viewport_height: usize,
    pub pending_workflow: Option<workflow::WorkflowFolder>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            jobs: Vec::new(),
            job_entries: Vec::new(),
            selected_job_index: None,
            scroll_offset: 0,
            rx: None,
            cancel_tx: None,
            is_executing_workflow: false,
            workflow_temp_dirs: Arc::new(Mutex::new(HashMap::new())),
            auto_scroll_enabled: true,
            last_viewport_width: 80,
            last_viewport_height: 20,
            pending_workflow: None,
        }
    }
}

fn create_tmp_workflow_folder<P: AsRef<Path>>(source_path: P) -> Result<TempDir, std::io::Error> {
    // Create temp directory
    let now = chrono::Local::now();
    let timestamp = now.format("%Y-%m-%d-%H-%M-%S").to_string();
    let prefix = format!("silva-{timestamp}-");
    let temp_dir = tempfile::Builder::new().prefix(&prefix).tempdir()?;

    // Copy source folder contents to temp directory
    let mut options = fs_extra::dir::CopyOptions::new();
    options.overwrite = true;
    options.copy_inside = true; // Copy contents, not the folder itself
    options.content_only = true;

    fs_extra::dir::copy(source_path, temp_dir.path(), &options)
        .map_err(|e| std::io::Error::other(format!("copy folder error {e}")))?;

    Ok(temp_dir)
}

impl State {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn handle_input(&mut self, key: KeyEvent) {
        let shift = key.modifiers.contains(KeyModifiers::SHIFT);

        match key.code {
            KeyCode::Down | KeyCode::Char('j') if !self.jobs.is_empty() && !shift => {
                self.select_next_job()
            }
            KeyCode::Up | KeyCode::Char('k') if !self.jobs.is_empty() && !shift => {
                self.select_previous_job()
            }
            KeyCode::Down | KeyCode::Char('j') if shift => self.scroll_down(),
            KeyCode::Up | KeyCode::Char('k') if shift => {
                self.scroll_up();
                self.auto_scroll_enabled = false;
            }
            KeyCode::PageUp => {
                self.scroll_up();
                self.auto_scroll_enabled = false;
            }
            KeyCode::PageDown => self.scroll_down(),
            KeyCode::Char('b') => self.scroll_to_bottom(),
            KeyCode::Char('r') if !self.is_executing_workflow => self.run_workflow(),
            _ => {}
        }
    }

    pub fn update(&mut self) {
        if let Some(rx) = self.rx.as_mut()
            && let Ok((idx, status, log_line)) = rx.try_recv()
        {
            if let Some(job_entry) = self.job_entries.get_mut(idx) {
                job_entry.status = status;
                job_entry.logs.push(log_line);

                // Auto-scroll to bottom if enabled and this is the selected job
                if self.auto_scroll_enabled && self.selected_job_index == Some(idx) {
                    self.scroll_to_bottom();
                }
            } else {
                // idx == jobs.len()
                self.is_executing_workflow = false;
            }
        }
    }

    pub fn run_workflow(&mut self) {
        // Get the pending workflow, return early if none
        let workflow_folder = match self.pending_workflow.take() {
            Some(wf) => wf,
            None => return,
        };

        self.is_executing_workflow = true;
        self.select_next_job();

        let (tx, rx) = mpsc::channel::<(usize, JobStatus, LogLine)>(32);
        let (cancel_tx, mut cancel_rx) = mpsc::channel::<()>(1);
        self.rx = Some(rx);
        self.cancel_tx = Some(cancel_tx);
        let jobs = self.jobs.clone();
        let workflow_temp_dirs = self.workflow_temp_dirs.clone();

        tokio::spawn(async move {
            let mut docker_executor = match DockerExecutor::new(tx.clone()) {
                Ok(docker_executor) => docker_executor,
                Err(e) => {
                    let log_line = LogLine::new(
                        LogSource::Stderr,
                        format!("Faild to create docker executor: {e}"),
                    );
                    tx.send((0, JobStatus::Failed, log_line)).await.unwrap();
                    // run workflow completes
                    tx.send((jobs.len(), JobStatus::Failed, LogLine::empty()))
                        .await
                        .unwrap();
                    return;
                }
            };

            // Create a temp workflow path
            let temp_workflow_dir = match create_tmp_workflow_folder(&workflow_folder.path) {
                Ok(temp_dir) => {
                    let temp_dir_path = temp_dir.path().to_path_buf();
                    let workflow_temp_dirs_arc = workflow_temp_dirs.clone();
                    let mut workflow_temp_dirs = workflow_temp_dirs_arc.lock().unwrap();
                    let workflow_dir_vec = workflow_temp_dirs
                        .entry(workflow_folder.name.to_string())
                        .or_default();
                    workflow_dir_vec.push(temp_dir);
                    temp_dir_path
                }
                Err(e) => {
                    let log_line =
                        LogLine::new(LogSource::Stderr, format!("Create temp dir error: {e}"));
                    tx.send((jobs.len(), JobStatus::Failed, log_line))
                        .await
                        .unwrap();
                    return;
                }
            };

            // Sort jobs in dependency order (topological sort)
            let sorted_jobs = match topological_sort_jobs(&jobs) {
                Ok(sorted) => {
                    let log_line = LogLine::new(
                        LogSource::Stdout,
                        format!(
                            "Jobs will execute in dependency order: {}",
                            sorted
                                .iter()
                                .map(|j| j.name.as_str())
                                .collect::<Vec<_>>()
                                .join(" → ")
                        ),
                    );
                    tx.send((0, JobStatus::Idle, log_line)).await.unwrap();
                    sorted
                }
                Err(e) => {
                    let log_line =
                        LogLine::new(LogSource::Stderr, format!("Dependency error: {e}"));
                    tx.send((0, JobStatus::Failed, log_line)).await.unwrap();
                    tx.send((jobs.len(), JobStatus::Failed, LogLine::empty()))
                        .await
                        .unwrap();
                    return;
                }
            };

            // Execute jobs sequentially in dependency order
            let jobs_length = jobs.len();

            // Create a map from job name to original index for UI updates
            let job_name_to_idx: HashMap<String, usize> = jobs
                .iter()
                .enumerate()
                .map(|(idx, job)| (job.name.clone(), idx))
                .collect();

            // Container registry: image_name -> container_id (for reusing containers)
            let mut container_registry: HashMap<String, String> = HashMap::new();

            for job in sorted_jobs.iter() {
                // Get the original index for this job (for UI updates)
                let idx = *job_name_to_idx.get(&job.name).unwrap();

                match job.load_config() {
                    Ok(config) => {
                        docker_executor.set_job_idx(idx);

                        // Load job parameters (if they exist)
                        let params = job.load_params().ok().flatten().unwrap_or_default();

                        // Copy input files from dependencies before running the job
                        copy_input_files_from_dependencies(
                            &temp_workflow_dir,
                            job,
                            &jobs,
                            &config,
                            &tx,
                            idx,
                        )
                        .await;

                        match docker_executor
                            .run_job(
                                &workflow_folder.name,
                                &temp_workflow_dir,
                                job,
                                &config,
                                &params,
                                &mut container_registry,
                                &mut cancel_rx,
                            )
                            .await
                        {
                            Ok(container_id) => {
                                // Container is tracked in the registry and will be cleaned up at the end
                                let _ = container_id; // Suppress unused variable warning
                            }
                            Err(e) => {
                                let log_line = LogLine::new(
                                    LogSource::Stderr,
                                    format!("docker run job error: {e}"),
                                );
                                tx.send((idx, JobStatus::Failed, log_line)).await.unwrap();
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        let log_line =
                            LogLine::new(LogSource::Stderr, format!("Load job config error: {e}"));
                        tx.send((idx, JobStatus::Failed, log_line)).await.unwrap();
                        break;
                    }
                }
            }

            // Cleanup all containers after workflow completes (success or failure)
            let container_ids: Vec<String> = container_registry.values().cloned().collect();
            docker_executor.cleanup_containers(&container_ids).await;

            // run workflow completes
            tx.send((jobs_length, JobStatus::Completed, LogLine::empty()))
                .await
                .unwrap();
        });
    }

    /// Calculate the total number of wrapped lines for the current job's logs
    fn calculate_wrapped_line_count(&self) -> usize {
        if let Some(job) = self.get_selected_job_entry() {
            let available_width = self.last_viewport_width.saturating_sub(17); // prefix width
            if available_width <= 10 {
                return job.logs.lines().len();
            }

            let mut total_lines = 0;
            for log_line in job.logs.lines() {
                // Use the same wrapping logic as rendering to ensure accurate count
                let wrapped_lines = textwrap::wrap(&log_line.content, available_width);
                total_lines += wrapped_lines.len();
            }
            total_lines
        } else {
            0
        }
    }

    pub fn scroll_up(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset = self.scroll_offset.saturating_sub(1);
        }
    }

    pub fn scroll_down(&mut self) {
        let wrapped_line_count = self.calculate_wrapped_line_count();
        let max_scroll = wrapped_line_count.saturating_sub(self.last_viewport_height);
        if self.scroll_offset < max_scroll {
            self.scroll_offset += 1;
        } else {
            // At bottom, re-enable auto-scroll
            self.auto_scroll_enabled = true;
        }
    }

    pub fn scroll_to_bottom(&mut self) {
        let wrapped_line_count = self.calculate_wrapped_line_count();
        self.scroll_offset = wrapped_line_count.saturating_sub(self.last_viewport_height);
        self.auto_scroll_enabled = true;
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Gets the currently selected job.
    pub fn get_selected_job_entry(&self) -> Option<&JobEntry> {
        self.selected_job_index
            .and_then(|idx| self.job_entries.get(idx))
    }

    /// Gets a mutable reference to the currently selected job.
    pub fn get_selected_job_mut(&mut self) -> Option<&mut Job> {
        self.selected_job_index
            .and_then(|idx| self.jobs.get_mut(idx))
    }

    /// Gets a job by index.
    pub fn get_job(&self, index: usize) -> Option<&Job> {
        self.jobs.get(index)
    }

    /// Gets a mutable reference to a job by index.
    pub fn get_job_mut(&mut self, index: usize) -> Option<&mut Job> {
        self.jobs.get_mut(index)
    }

    /// Selects the next job in the list.
    pub fn select_next_job(&mut self) {
        if self.jobs.is_empty() {
            self.selected_job_index = None;
            return;
        }

        self.selected_job_index = Some(match self.selected_job_index {
            Some(idx) => (idx + 1) % self.jobs.len(),
            None => 0,
        });

        // Reset scroll and re-enable auto-scroll when changing jobs
        self.scroll_offset = 0;
        self.auto_scroll_enabled = true;
    }

    /// Selects the previous job in the list.
    pub fn select_previous_job(&mut self) {
        if self.jobs.is_empty() {
            self.selected_job_index = None;
            return;
        }

        self.selected_job_index = Some(match self.selected_job_index {
            Some(idx) => {
                if idx > 0 {
                    idx - 1
                } else {
                    self.jobs.len() - 1
                }
            }
            None => self.jobs.len() - 1,
        });

        // Reset scroll and re-enable auto-scroll when changing jobs
        self.scroll_offset = 0;
        self.auto_scroll_enabled = true;
    }

    /// Clears all jobs from the list.
    pub fn clear_jobs(&mut self) {
        self.jobs.clear();
        self.job_entries.clear();
        self.selected_job_index = None;
        self.scroll_offset = 0;
    }
}

/// Performs topological sort on jobs based on their dependencies.
///
/// # Arguments
///
/// * `jobs` - List of jobs to sort
///
/// # Returns
///
/// * `Ok(Vec<Job>)` - Jobs sorted in dependency order (dependencies first)
/// * `Err(String)` - Error message if circular dependency detected or invalid dependency
///
/// # Algorithm
///
/// Uses Kahn's algorithm for topological sorting:
/// 1. Build dependency graph and calculate in-degrees
/// 2. Start with jobs that have no dependencies (in-degree = 0)
/// 3. Process jobs in order, removing edges as we go
/// 4. If we can't process all jobs, there's a cycle
fn topological_sort_jobs(jobs: &[Job]) -> Result<Vec<Job>, String> {
    use std::collections::{HashMap, VecDeque};

    if jobs.is_empty() {
        return Ok(Vec::new());
    }

    // Build a map of job names to job data for quick lookup
    let job_map: HashMap<String, Job> = jobs.iter().map(|j| (j.name.clone(), j.clone())).collect();

    // Build dependency graph: job_name -> Vec<jobs that depend on it>
    let mut dependents: HashMap<String, Vec<String>> = HashMap::new();
    let mut in_degree: HashMap<String, usize> = HashMap::new();

    // Initialize all jobs with in-degree 0
    for job in jobs {
        in_degree.insert(job.name.clone(), 0);
        dependents.entry(job.name.clone()).or_default();
    }

    // Build the graph
    for job in jobs {
        let config = match job.load_config() {
            Ok(cfg) => cfg,
            Err(e) => {
                return Err(format!(
                    "Failed to load config for job '{}': {}",
                    job.name, e
                ));
            }
        };

        for dep_name in &config.depends_on {
            // Validate that the dependency exists
            if !job_map.contains_key(dep_name) {
                return Err(format!(
                    "Job '{}' depends on '{}', but '{}' does not exist in the workflow",
                    job.name, dep_name, dep_name
                ));
            }

            // Add edge: dep_name -> job.name
            dependents
                .entry(dep_name.clone())
                .or_default()
                .push(job.name.clone());

            // Increment in-degree of current job
            *in_degree.get_mut(&job.name).unwrap() += 1;
        }
    }

    // Kahn's algorithm: start with jobs that have no dependencies
    let mut queue: VecDeque<String> = in_degree
        .iter()
        .filter(|&(_, &degree)| degree == 0)
        .map(|(name, _)| name.clone())
        .collect();

    let mut sorted_jobs = Vec::new();

    while let Some(job_name) = queue.pop_front() {
        // Add this job to the sorted list
        if let Some(job) = job_map.get(&job_name) {
            sorted_jobs.push(job.clone());
        }

        // Process all jobs that depend on this one
        if let Some(deps) = dependents.get(&job_name) {
            for dependent in deps {
                // Decrease in-degree
                let degree = in_degree.get_mut(dependent).unwrap();
                *degree -= 1;

                // If in-degree becomes 0, add to queue
                if *degree == 0 {
                    queue.push_back(dependent.clone());
                }
            }
        }
    }

    // Check if all jobs were processed (no cycles)
    if sorted_jobs.len() != jobs.len() {
        // Find jobs that are part of the cycle
        let processed: std::collections::HashSet<_> = sorted_jobs.iter().map(|j| &j.name).collect();
        let unprocessed: Vec<_> = jobs
            .iter()
            .filter(|j| !processed.contains(&j.name))
            .map(|j| j.name.as_str())
            .collect();

        return Err(format!(
            "Circular dependency detected involving jobs: {}",
            unprocessed.join(", ")
        ));
    }

    Ok(sorted_jobs)
}

/// Helper function to recursively copy a directory and all its contents.
///
/// # Arguments
///
/// * `src` - Source directory to copy from
/// * `dst` - Destination directory to copy to
///
/// # Returns
///
/// Returns the number of files copied, or an error if the copy fails.
fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<usize> {
    use std::fs;

    // Create the destination directory if it doesn't exist
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }

    let mut file_count = 0;

    // Iterate through entries in the source directory
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let dest_path = dst.join(entry.file_name());

        if path.is_dir() {
            // Recursively copy subdirectory
            file_count += copy_dir_recursive(&path, &dest_path)?;
        } else {
            // Copy file
            fs::copy(&path, &dest_path)?;
            file_count += 1;
        }
    }

    Ok(file_count)
}

/// Helper function to copy input files from dependency jobs' outputs to the current job folder.
///
/// # Arguments
///
/// * `temp_workflow_dir` - The temporary workflow directory containing all job folders
/// * `current_job` - The current job that needs input files
/// * `all_jobs` - List of all jobs in the workflow (to look up dependencies by name)
/// * `config` - The job configuration containing dependencies and input patterns
/// * `tx` - Message channel for logging
/// * `job_idx` - Current job index for message tagging
///
/// # Behavior
///
/// For each dependency specified in config.depends_on:
/// - Looks for the dependency's outputs/ folder
/// - If config.inputs is specified: copies only matching files (supports globs)
/// - If config.inputs is empty: copies all files from dependency outputs
/// - Handles conflicts: uses first match and sends warning
async fn copy_input_files_from_dependencies(
    temp_workflow_dir: &Path,
    current_job: &Job,
    all_jobs: &[Job],
    config: &job_config::config::JobConfig,
    tx: &mpsc::Sender<(usize, JobStatus, LogLine)>,
    job_idx: usize,
) {
    use std::collections::HashSet;
    use std::fs;
    use std::path::PathBuf;

    if config.depends_on.is_empty() {
        return;
    }

    let current_job_dir = temp_workflow_dir.join(&current_job.name);
    let mut copied_files: HashSet<String> = HashSet::new();

    for dep_job_name in &config.depends_on {
        // Find the dependency job
        let dep_job = all_jobs.iter().find(|j| &j.name == dep_job_name);
        if dep_job.is_none() {
            let log_line = LogLine::new(
                LogSource::Stderr,
                format!("Warning: Dependency job '{dep_job_name}' not found"),
            );
            let _ = tx.send((job_idx, JobStatus::Running, log_line)).await;
            continue;
        }

        let dep_outputs_dir = temp_workflow_dir.join(dep_job_name).join("outputs");
        if !dep_outputs_dir.exists() {
            let log_line = LogLine::new(
                LogSource::Stdout,
                format!("No outputs found for dependency '{dep_job_name}', skipping"),
            );
            let _ = tx.send((job_idx, JobStatus::Running, log_line)).await;
            continue;
        }

        // Determine which files to copy
        let files_to_copy: Vec<PathBuf> = if config.inputs.is_empty() {
            // Copy all files from outputs/
            match fs::read_dir(&dep_outputs_dir) {
                Ok(entries) => entries.filter_map(|e| e.ok()).map(|e| e.path()).collect(),
                Err(e) => {
                    let log_line = LogLine::new(
                        LogSource::Stderr,
                        format!("Error reading outputs from '{dep_job_name}': {e}"),
                    );
                    let _ = tx.send((job_idx, JobStatus::Running, log_line)).await;
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
                        let log_line = LogLine::new(
                            LogSource::Stderr,
                            format!("Invalid glob pattern '{pattern}': {e}"),
                        );
                        let _ = tx.send((job_idx, JobStatus::Running, log_line)).await;
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
                    let log_line = LogLine::new(
                        LogSource::Stderr,
                        format!(
                            "Warning: File '{filename_str}' already copied from another dependency, skipping from '{dep_job_name}'"
                        ),
                    );
                    let _ = tx.send((job_idx, JobStatus::Running, log_line)).await;
                    continue;
                }

                // Copy the file or directory
                if source_path.is_file() {
                    // Copy single file
                    match fs::copy(&source_path, &dest_path) {
                        Ok(_) => {
                            copied_files.insert(filename_str.clone());
                            let log_line = LogLine::new(
                                LogSource::Stdout,
                                format!("Copied file '{filename_str}' from '{dep_job_name}'"),
                            );
                            let _ = tx.send((job_idx, JobStatus::Running, log_line)).await;
                        }
                        Err(e) => {
                            let log_line = LogLine::new(
                                LogSource::Stderr,
                                format!(
                                    "Error copying file '{filename_str}' from '{dep_job_name}': {e}"
                                ),
                            );
                            let _ = tx.send((job_idx, JobStatus::Running, log_line)).await;
                        }
                    }
                } else if source_path.is_dir() {
                    // Copy directory recursively
                    match copy_dir_recursive(&source_path, &dest_path) {
                        Ok(file_count) => {
                            copied_files.insert(filename_str.clone());
                            let log_line = LogLine::new(
                                LogSource::Stdout,
                                format!(
                                    "Copied directory '{filename_str}/' ({file_count} file(s)) from '{dep_job_name}'"
                                ),
                            );
                            let _ = tx.send((job_idx, JobStatus::Running, log_line)).await;
                        }
                        Err(e) => {
                            let log_line = LogLine::new(
                                LogSource::Stderr,
                                format!(
                                    "Error copying directory '{filename_str}/' from '{dep_job_name}': {e}"
                                ),
                            );
                            let _ = tx.send((job_idx, JobStatus::Running, log_line)).await;
                        }
                    }
                } else {
                    // Not a regular file or directory (e.g., symlink)
                    let log_line = LogLine::new(
                        LogSource::Stderr,
                        format!(
                            "Warning: Skipping '{filename_str}' from '{dep_job_name}' (not a regular file or directory)"
                        ),
                    );
                    let _ = tx.send((job_idx, JobStatus::Running, log_line)).await;
                }
            }
        }
    }

    if !copied_files.is_empty() {
        let log_line = LogLine::new(
            LogSource::Stdout,
            format!(
                "Copied {} input file(s) from dependencies",
                copied_files.len()
            ),
        );
        let _ = tx.send((job_idx, JobStatus::Running, log_line)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::docker::job::JobStatus;

    #[test]
    fn test_job_status_as_str() {
        assert_eq!(JobStatus::Idle.as_str(), "Idle");
        assert_eq!(JobStatus::BuildingImage.as_str(), "Building Image");
        assert_eq!(JobStatus::Completed.as_str(), "Completed");
    }

    #[test]
    fn test_job_status_is_running() {
        assert!(!JobStatus::Idle.is_running());
        assert!(JobStatus::PullingImage.is_running());
        assert!(JobStatus::Running.is_running());
        assert!(!JobStatus::Completed.is_running());
        assert!(!JobStatus::Failed.is_running());
    }

    #[test]
    fn test_job_status_is_finished() {
        assert!(!JobStatus::Idle.is_finished());
        assert!(!JobStatus::Running.is_finished());
        assert!(JobStatus::Completed.is_finished());
        assert!(JobStatus::Failed.is_finished());
    }

    #[test]
    fn test_docker_job_state_scroll() {
        let mut state = State::new();

        // Scrolling up from 0 should do nothing
        state.scroll_up();
        assert_eq!(state.scroll_offset, 0);

        // Scroll down
        state.scroll_down();
        assert_eq!(state.scroll_offset, 0); // No logs yet, so stays at 0
    }
}

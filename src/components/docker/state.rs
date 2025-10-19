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
    pub auto_scroll_enabled: bool,
    pub last_viewport_width: usize,
    pub last_viewport_height: usize,
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
            auto_scroll_enabled: true,
            last_viewport_width: 80,
            last_viewport_height: 20,
        }
    }
}

fn copy_to_temp_dir<P: AsRef<Path>>(source_path: P) -> Result<TempDir, std::io::Error> {
    // Create temp directory

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

    pub fn run_workflow(&mut self, workflow_folder: workflow::WorkflowFolder) {
        self.is_executing_workflow = true;
        self.select_next_job();

        let (tx, rx) = mpsc::channel::<(usize, JobStatus, LogLine)>(32);
        let (cancel_tx, mut cancel_rx) = mpsc::channel::<()>(1);
        self.rx = Some(rx);
        self.cancel_tx = Some(cancel_tx);
        let jobs = self.jobs.clone();

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
            let now = chrono::Local::now();
            let timestamp = now.format("%Y-%m-%d-%H-%M-%S").to_string();
            let prefix = format!("silva-{timestamp}-");
            let temp_dir = tempfile::Builder::new().prefix(prefix).tempdir()?;

            // Copy source folder contents to temp directory
            let mut options = fs_extra::dir::CopyOptions::new();
            options.overwrite = true;
            options.copy_inside = true; // Copy contents, not the folder itself
            options.content_only = true;

            fs_extra::dir::copy(source_path, temp_dir.path(), &options)
                .map_err(|e| std::io::Error::other(format!("copy folder error {e}")))?;

            // Execute jobs sequentially
            let jobs_length = jobs.len();
            for (idx, job) in jobs.into_iter().enumerate() {
                match job.load_config() {
                    Ok(config) => {
                        let temp_dir_path = match copy_to_temp_dir(&job.path) {
                            Ok(temp_dir) => {
                                let temp_dir_path = temp_dir.path().to_path_buf();
                                let job_temp_dirs_arc = job_temp_dirs.clone();
                                let mut job_temp_dirs = job_temp_dirs_arc.lock().unwrap();
                                job_temp_dirs.insert(job.name.to_string(), temp_dir);
                                temp_dir_path
                            }
                            Err(e) => {
                                let log_line = LogLine::new(
                                    LogSource::Stderr,
                                    format!("Create temp dir error: {e}"),
                                );
                                tx.send((idx, JobStatus::Failed, log_line)).await.unwrap();
                                break;
                            }
                        };

                        docker_executor.set_job_idx(idx);

                        match docker_executor
                            .run_job(&temp_dir_path, &config, &mut cancel_rx)
                            .await
                        {
                            Ok(_) => (),
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

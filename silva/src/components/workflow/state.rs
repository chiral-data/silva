use crossterm::event::{KeyCode, KeyEvent};

use crate::components::docker;

pub struct State {
    pub selected_workflow: Option<usize>,
    pub workflow_manager: super::WorkflowManager,
    pub docker_state: docker::state::State,
    pub show_docker_popup: bool,
}

impl Default for State {
    fn default() -> Self {
        // Initialize workflow manager
        let home = super::WorkflowHome::new().unwrap_or_else(|e| {
            eprintln!("Warning: Failed to initialize workflow home: {e}");
            // Fall back to a default path
            super::WorkflowHome::new().unwrap()
        });

        let mut workflow_manager = super::WorkflowManager::new(home);
        if let Err(e) = workflow_manager.initialize() {
            eprintln!("Warning: Failed to initialize workflow manager: {e}");
        }

        Self {
            selected_workflow: None,
            workflow_manager,
            docker_state: docker::state::State::default(),
            show_docker_popup: false,
        }
    }
}

impl State {
    pub async fn handle_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('d') => self.toggle_docker_popup(),
            _ => {
                if self.show_docker_popup {
                    self.docker_state.handle_input(key);
                } else if !self.docker_state.is_executing_workflow {
                    match key.code {
                        KeyCode::Char('r') => self.refresh_workflows(),
                        KeyCode::Up | KeyCode::Char('j') => self.select_previous_workflow(),
                        KeyCode::Down | KeyCode::Char('k') => self.select_next_workflow(),
                        KeyCode::Enter => {
                            if let Some(workflow_folder) = self.get_selected_workflow() {
                                self.docker_state.run_workflow(workflow_folder.to_owned());
                                self.toggle_docker_popup();
                            }
                        }
                        _ => (),
                    }
                }
            }
        }
    }

    pub fn get_selected_workflow(&self) -> Option<&super::WorkflowFolder> {
        if let Some(idx) = self.selected_workflow {
            self.workflow_manager.get_workflows().get(idx)
        } else {
            None
        }
    }

    pub fn toggle_docker_popup(&mut self) {
        self.show_docker_popup = !self.show_docker_popup;
    }

    pub fn refresh_workflows(&mut self) {
        if let Err(e) = self.workflow_manager.refresh() {
            eprintln!("Error refreshing workflows: {e}");
        }
    }

    fn scan_jobs(&mut self) {
        if let Some(wf_sel) = self.get_selected_workflow() {
            // Scan for jobs in the selected workflow
            if let Ok(jobs) = super::JobScanner::scan_jobs(&wf_sel.path) {
                self.docker_state.clear_jobs();
                self.docker_state.job_entries = jobs
                    .iter()
                    .map(|job| docker::job::JobEntry::new(job.name.to_string()))
                    .collect();
                self.docker_state.jobs = jobs;
            }
        }
    }

    pub fn select_next_workflow(&mut self) {
        let count = self.workflow_manager.count();
        if count == 0 {
            self.selected_workflow = None;
            return;
        }

        self.selected_workflow = Some(match self.selected_workflow {
            Some(idx) => (idx + 1) % count,
            None => 0,
        });

        self.scan_jobs();
    }

    pub fn select_previous_workflow(&mut self) {
        let count = self.workflow_manager.count();
        if count == 0 {
            self.selected_workflow = None;
            return;
        }

        self.selected_workflow = Some(match self.selected_workflow {
            Some(idx) => {
                if idx > 0 {
                    idx - 1
                } else {
                    count - 1
                }
            }
            None => count - 1,
        });

        self.scan_jobs();
    }
}

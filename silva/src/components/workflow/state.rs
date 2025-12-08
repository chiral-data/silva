use crossterm::event::{KeyCode, KeyEvent};

use crate::components::docker;

use super::{JobParamSource, ParamsEditorState, WorkflowParamSource};

pub struct State {
    pub selected_workflow: Option<usize>,
    pub workflow_manager: super::WorkflowManager,
    pub docker_state: docker::state::State,
    pub show_docker_popup: bool,
    pub show_params_popup: bool,
    pub params_editor_state: Option<ParamsEditorState<JobParamSource>>,
    pub show_global_params_popup: bool,
    pub global_params_editor_state: Option<ParamsEditorState<WorkflowParamSource>>,
}

impl Default for State {
    fn default() -> Self {
        // Initialize workflow manager
        let home = super::WorkflowHome::new().unwrap_or_else(|_e| {
            // Fall back to a default path
            super::WorkflowHome::new().unwrap()
        });

        let mut workflow_manager = super::WorkflowManager::new(home);
        let _ = workflow_manager.initialize();

        Self {
            selected_workflow: None,
            workflow_manager,
            docker_state: docker::state::State::default(),
            show_docker_popup: false,
            show_params_popup: false,
            params_editor_state: None,
            show_global_params_popup: false,
            global_params_editor_state: None,
        }
    }
}

impl State {
    pub async fn handle_input(&mut self, key: KeyEvent) {
        // Handle global params popup input first if it's open
        if self.show_global_params_popup {
            self.handle_global_params_editor_input(key);
            return;
        }

        // Handle params popup input if it's open
        if self.show_params_popup {
            self.handle_params_editor_input(key);
            return;
        }

        match key.code {
            KeyCode::Char('d') => self.toggle_docker_popup(),
            KeyCode::Char('p') => self.open_params_editor(),
            KeyCode::Char('g') => self.open_global_params_editor(),
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
                                self.docker_state.pending_workflow =
                                    Some(workflow_folder.to_owned());
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
        let _ = self.workflow_manager.refresh();
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

    /// Opens the parameter editor for the selected job.
    pub fn open_params_editor(&mut self) {
        // Need to have a selected workflow with jobs
        if self.docker_state.jobs.is_empty() {
            return;
        }

        // Get the selected job from docker_state
        if let Some(selected_job_idx) = self.docker_state.selected_job_index
            && let Some(job) = self.docker_state.jobs.get(selected_job_idx)
        {
            // Load job metadata
            let job_meta: job_config::job::JobMeta = match job.load_meta() {
                Ok(meta) => meta,
                Err(_e) => {
                    return;
                }
            };

            // Create params editor state using JobParamSource
            let source = JobParamSource::new(job.clone(), job_meta);
            match ParamsEditorState::new(source) {
                Ok(state) => {
                    self.params_editor_state = Some(state);
                    self.show_params_popup = true;
                }
                Err(_e) => {
                    return;
                }
            }
        }
    }

    /// Closes the parameter editor.
    pub fn close_params_editor(&mut self) {
        self.show_params_popup = false;
        self.params_editor_state = None;
    }

    /// Handles input for the parameter editor popup.
    fn handle_params_editor_input(&mut self, key: KeyEvent) {
        if let Some(editor_state) = &mut self.params_editor_state {
            if editor_state.editing {
                // In editing mode
                match key.code {
                    KeyCode::Char(c) => {
                        editor_state.input_char(c);
                    }
                    KeyCode::Backspace => {
                        editor_state.input_backspace();
                    }
                    KeyCode::Enter => {
                        editor_state.save_current_edit();
                    }
                    KeyCode::Esc => {
                        editor_state.cancel_editing();
                    }
                    _ => {}
                }
            } else {
                // Navigation mode
                match key.code {
                    KeyCode::Up | KeyCode::Char('j') => {
                        editor_state.move_up();
                    }
                    KeyCode::Down | KeyCode::Char('k') => {
                        editor_state.move_down();
                    }
                    KeyCode::Enter => {
                        editor_state.start_editing();
                    }
                    KeyCode::Char('s') => {
                        // Save all params and close
                        if let Err(e) = editor_state.save_params() {
                            editor_state.error_message = Some(e);
                        } else {
                            self.close_params_editor();
                        }
                    }
                    KeyCode::Esc => {
                        self.close_params_editor();
                    }
                    _ => {}
                }
            }
        }
    }

    /// Opens the global parameter editor for the selected workflow.
    pub fn open_global_params_editor(&mut self) {
        // Need to have a selected workflow
        if let Some(workflow_folder) = self.get_selected_workflow() {
            // Load or create workflow metadata
            let workflow_metadata: job_config::workflow::WorkflowMeta =
                match workflow_folder.load_workflow_metadata() {
                    Ok(Some(metadata)) => metadata,
                    Ok(None) => {
                        // Create default metadata
                        let metadata = job_config::workflow::WorkflowMeta::new(
                            workflow_folder.name.clone(),
                            "Global workflow parameters".to_string(),
                        );
                        // Save it for future use
                        let _ = workflow_folder.save_workflow_metadata(&metadata);
                        metadata
                    }
                    Err(_e) => {
                        return;
                    }
                };

            // Create global params editor state using WorkflowParamSource
            let source = WorkflowParamSource::new(workflow_folder.clone(), workflow_metadata);
            match ParamsEditorState::new(source) {
                Ok(state) => {
                    self.global_params_editor_state = Some(state);
                    self.show_global_params_popup = true;
                }
                Err(_e) => {}
            }
        }
    }

    /// Closes the global parameter editor.
    pub fn close_global_params_editor(&mut self) {
        self.show_global_params_popup = false;
        self.global_params_editor_state = None;
    }

    /// Handles input for the global parameter editor popup.
    fn handle_global_params_editor_input(&mut self, key: KeyEvent) {
        if let Some(editor_state) = &mut self.global_params_editor_state {
            if editor_state.editing {
                // In editing mode
                match key.code {
                    KeyCode::Char(c) => {
                        editor_state.input_char(c);
                    }
                    KeyCode::Backspace => {
                        editor_state.input_backspace();
                    }
                    KeyCode::Enter => {
                        editor_state.save_current_edit();
                    }
                    KeyCode::Esc => {
                        editor_state.cancel_editing();
                    }
                    _ => {}
                }
            } else {
                // Navigation mode
                match key.code {
                    KeyCode::Up | KeyCode::Char('j') => {
                        editor_state.move_up();
                    }
                    KeyCode::Down | KeyCode::Char('k') => {
                        editor_state.move_down();
                    }
                    KeyCode::Enter => {
                        editor_state.start_editing();
                    }
                    KeyCode::Char('s') => {
                        // Save all params and close
                        if let Err(e) = editor_state.save_params() {
                            editor_state.error_message = Some(e);
                        } else {
                            self.close_global_params_editor();
                        }
                    }
                    KeyCode::Esc => {
                        self.close_global_params_editor();
                    }
                    _ => {}
                }
            }
        }
    }
}

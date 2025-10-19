use crossterm::event;
use ratatui::prelude::*;

use crate::data_model;
use crate::ui;
use crate::ui::layout::info::MessageLevel;

pub fn render(f: &mut Frame, area: Rect, states: &mut ui::states::States, _store: &data_model::Store) {
    let current_style = states.get_style(true);
    ui::components::helper_hint::render_job_new(f, area, current_style);
}

pub fn handle_key(key: &event::KeyEvent, states: &mut ui::states::States, store: &mut data_model::Store) {
    use event::KeyCode;

    if key.code == KeyCode::Enter {
        if store.project_sel.is_none() {
            states.info_states.message = ("no project selected".to_string(), MessageLevel::Warn);
        } else {
            // Check if project has multiple job configurations
            if let Some((proj, _)) = store.project_sel.as_ref() {
                match data_model::job::Job::get_settings_vec(proj.get_dir()) {
                    Ok(settings_vec) => {
                        if settings_vec.len() == 1 {
                            // Single config - create job directly
                            let project_path = proj.get_dir().display().to_string();
                            let mut job_mgr = store.job_mgr.lock().unwrap();
                            let new_job_id = job_mgr.create_job(Some(project_path), Some(0));
                            drop(job_mgr);
                            
                            states.job_states.set_selected_job_id(Some(new_job_id));
                            states.tabs_states.tab = ui::layout::tabs::Tab::Job;
                            states.job_states.show_page = ui::pages::job::ShowPage::Detail;
                        } else {
                            // Multiple configs - go to config selection
                            states.tabs_states.tab = ui::layout::tabs::Tab::Job;
                            states.job_states.show_page = ui::pages::job::ShowPage::ConfigSelect;
                        }
                    }
                    Err(e) => {
                        states.info_states.message = (format!("Error reading job configurations: {}", e), MessageLevel::Error);
                    }
                }
            }
        }
    } 
}

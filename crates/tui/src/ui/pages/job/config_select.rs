use ratatui::prelude::*;
use ratatui::widgets::*;
use crossterm::event;

use crate::data_model;
use crate::ui;
use crate::ui::layout::info::MessageLevel;

#[derive(Default)]
pub struct States {
    pub config_list: ratatui::widgets::ListState,
    pub configs: Vec<data_model::job::settings::Settings>,
}

pub fn render(f: &mut Frame, area: Rect, states: &mut ui::states::States, store: &data_model::Store) {
    let current_style = states.get_style(true);
    let states_current = &mut states.job_states.config_select;

    // Check if we have a selected project
    if let Some((proj, _)) = store.project_sel.as_ref() {
        // Try to get job settings from the project
        match data_model::job::Job::get_settings_vec(proj.get_dir()) {
            Ok(settings_vec) => {
                states_current.configs = settings_vec;
                
                let config_strings: Vec<String> = states_current.configs.iter()
                    .enumerate()
                    .map(|(i, settings)| {
                        let config_name = if states_current.configs.len() == 1 {
                            "@job.toml".to_string()
                        } else {
                            format!("@job_{}.toml", i + 1)
                        };
                        
                        let infra_info = match (&settings.infra_local, &settings.dok) {
                            (Some(_), _) => "Local",
                            (None, Some(_)) => "DOK",
                            (None, None) => "Unknown",
                        };
                        
                        let files_info = format!("inputs: {}, outputs: {}", 
                            settings.files.inputs.len(),
                            settings.files.outputs.len());
                        
                        format!("{} [{}] ({})", config_name, infra_info, files_info)
                    })
                    .collect();

                let config_list = List::new(config_strings)
                    .block(Block::bordered().title(" Select Job Configuration "))
                    .direction(ListDirection::TopToBottom)
                    .highlight_style(Style::default().bg(Color::DarkGray))
                    .highlight_symbol("> ");

                let help_text = vec![
                    Line::from("Select a job configuration to create a new job:"),
                    Line::from("↑/↓: Navigate configurations"),
                    Line::from("Enter: Select configuration and create job"),
                    Line::from("Esc: Return to job list"),
                ];

                let help_paragraph = Paragraph::new(help_text)
                    .block(Block::bordered().title(" Help "))
                    .style(current_style);

                let layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(1), Constraint::Length(6)])
                    .split(area);

                f.render_stateful_widget(config_list, layout[0], &mut states_current.config_list);
                f.render_widget(help_paragraph, layout[1]);
            }
            Err(e) => {
                let error_text = vec![
                    Line::from("Error loading job configurations:"),
                    Line::from(format!("{}", e)),
                    Line::from(""),
                    Line::from("Press Esc to return to job list"),
                ];
                
                let error_paragraph = Paragraph::new(error_text)
                    .block(Block::bordered().title(" Error "))
                    .style(Style::default().fg(Color::Red));

                f.render_widget(error_paragraph, area);
            }
        }
    } else {
        let no_project_text = vec![
            Line::from("No project selected"),
            Line::from("Please select a project first"),
            Line::from(""),
            Line::from("Press Esc to return to job list"),
        ];
        
        let no_project_paragraph = Paragraph::new(no_project_text)
            .block(Block::bordered().title(" No Project "))
            .style(Style::default().fg(Color::Yellow));

        f.render_widget(no_project_paragraph, area);
    }
}

pub fn handle_key(key: &event::KeyEvent, states: &mut ui::states::States, store: &data_model::Store) {
    use event::KeyCode;

    match key.code {
        KeyCode::Up => {
            let states_current = &mut states.job_states.config_select;
            if !states_current.configs.is_empty() {
                let i = match states_current.config_list.selected() {
                    Some(i) => {
                        if i == 0 {
                            states_current.configs.len() - 1
                        } else {
                            i - 1
                        }
                    }
                    None => 0,
                };
                states_current.config_list.select(Some(i));
            }
        }
        KeyCode::Down => {
            let states_current = &mut states.job_states.config_select;
            if !states_current.configs.is_empty() {
                let i = match states_current.config_list.selected() {
                    Some(i) => {
                        if i >= states_current.configs.len() - 1 {
                            0
                        } else {
                            i + 1
                        }
                    }
                    None => 0,
                };
                states_current.config_list.select(Some(i));
            }
        }
        KeyCode::Enter => {
            let states_current = &mut states.job_states.config_select;
            
            if let Some(selected_idx) = states_current.config_list.selected() {
                if let Some((proj, _)) = store.project_sel.as_ref() {
                    // Create a new job with the selected configuration
                    let project_path = proj.get_dir().display().to_string();
                    let mut job_mgr = store.job_mgr.lock().unwrap();
                    let new_job_id = job_mgr.create_job(Some(project_path), Some(selected_idx));
                    drop(job_mgr);
                    
                    states.job_states.set_selected_job_id(Some(new_job_id));
                    states.job_states.show_page = super::ShowPage::Detail;
                    
                    states.info_states.message = (
                        format!("Created job {} with configuration {}", new_job_id, selected_idx + 1),
                        MessageLevel::Info
                    );
                } else {
                    states.info_states.message = ("No project selected".to_string(), MessageLevel::Warn);
                }
            } else {
                states.info_states.message = ("No configuration selected".to_string(), MessageLevel::Warn);
            }
        }
        KeyCode::Esc => {
            states.job_states.show_page = super::ShowPage::List;
        }
        _ => {}
    }
}
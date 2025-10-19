use ratatui::prelude::*;
use ratatui::widgets::*;
use crossterm::event;

use crate::data_model;
use crate::ui;
use crate::ui::layout::info::MessageLevel;

#[derive(Default, PartialEq)]
pub enum Tab {
    Pod,
    #[default]
    Status,
    Dependencies,
    Files,
    // Build,
    Pre,
    Run,
    Cancel,
    Post,
    Chat,
}

impl Tab {
    fn texts(&self) -> (&str, &str) {
        match self {
            Self::Pod => ("Select a Pod", "[P]ods"), 
            Self::Status => ("Status Dashboard", "[S]tatus"),
            Self::Dependencies => ("Dependencies", "[D]eps"),
            Self::Files => ("Files", "[F]iles"), 
            Self::Pre => ("Pre-processing", "Pr[e]"),
            Self::Run => ("Run", "[R]un"),
            Self::Cancel => ("Cancel", "C[a]ncel"),
            Self::Post => ("Post-processing", "Po[s]t"),
            Self::Chat => ("Chat with LLM", "[C]hat"),
        }
    }

    fn index(&self) -> usize {
        match self {
            Tab::Pod => 0,
            Tab::Status => 1,
            Tab::Dependencies => 2,
            Tab::Files => 3,
            Tab::Pre => 4,
            Tab::Run => 5,
            Tab::Cancel => 6,
            Tab::Post => 7,
            Tab::Chat => 8,
        }
    }
}

#[derive(Default)]
pub struct States {
    // job_settings: data_model::job::settings::Settings,
    tab_action: Tab,
    list_state_file: ListState,
    pub chat: chat::States,
    pub status: status::States,
    pub dependencies: dependencies::States,
}

// impl States {
// }


// fn filter_tabs(tab: &Tab, states: &ui::states::States) -> bool {
//     match tab {
//         // build action not for localhost
//         Tab::Build => states.job_states.pod_type.pod_type_sel_id != Some(0),
//         _ => true
//     }
// }

pub fn render(f: &mut Frame, area: Rect, states: &mut ui::states::States, store: &mut data_model::Store) {
    let current_style = states.get_style(true);
    
    // Get job information for header
    let job_info = if let Some(job_id) = states.job_states.get_selected_job_id() {
        let job_mgr = store.job_mgr.lock().unwrap();
        if let Some(job) = job_mgr.jobs.get(&job_id) {
            let config_name = match job.config_index {
                Some(index) => {
                    if let Some((proj, _)) = store.project_sel.as_ref() {
                        match data_model::job::Job::get_settings_vec(proj.get_dir()) {
                            Ok(settings_vec) => {
                                if settings_vec.len() == 1 {
                                    "@job.toml".to_string()
                                } else {
                                    format!("@job_{}.toml", index + 1)
                                }
                            }
                            Err(_) => "Unknown".to_string(),
                        }
                    } else {
                        "Unknown".to_string()
                    }
                }
                None => "Unknown".to_string(),
            };
            
            format!("Job {} [{}] - Status: {}", job_id, config_name, job.status)
        } else {
            "No job selected".to_string()
        }
    } else {
        "No job selected".to_string()
    };

    let tabs_strings: Vec<String> = [
            Tab::Pod, Tab::Status, Tab::Dependencies, Tab::Files, Tab::Pre, Tab::Run, Tab::Cancel, Tab::Post, Tab::Chat
        ].into_iter()
        // .filter(|t| filter_tabs(t, states))
        .map(|t| {
            let texts = t.texts();
            if t == states.job_states.detail.tab_action {
                if matches!(t, Tab::Files | Tab::Chat | Tab::Status | Tab::Dependencies) { texts.0.to_string() } else { format!("[Enter] {}", texts.0) }
            } else { texts.1.to_string() }
        })
        .collect();
    let states_current = &mut states.job_states.detail;
    let actions = Tabs::new(tabs_strings)
        .block(Block::bordered().title(" Actions "))
        .select(states_current.tab_action.index())
        .divider(" ")
        .style(current_style);
    let helper_lines: Vec<Line> = match states_current.tab_action {
        Tab::Pod => pod::HELPER, 
        Tab::Status => status::HELPER,
        Tab::Dependencies => dependencies::HELPER,
        Tab::Files => files::HELPER,
        // Tab::Build => build::HELPER,
        Tab::Pre => pre::HELPER,
        Tab::Run => run::HELPER,
        Tab::Cancel => cancel::HELPER,
        Tab::Post => post::HELPER,
        Tab::Chat => chat::HELPER,
    }.iter()
        .map(|&s| Line::from(s))
        .collect();
    let helper = Paragraph::new(helper_lines)
        .style(current_style)
        .block(Block::bordered())
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });

    let job_info_widget = Paragraph::new(job_info)
        .block(Block::bordered().title(" Job Information "))
        .style(current_style);
    
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Length(3), Constraint::Max(5), Constraint::Min(1)])
        .split(area);
    let (job_info_area, actions_area, helper_area, content_area) = (layout[0], layout[1], layout[2], layout[3]);

    f.render_widget(job_info_widget, job_info_area);
    f.render_widget(actions, actions_area);
    f.render_widget(helper, helper_area);
    match states_current.tab_action {
        Tab::Pod => (),
        Tab::Status => status::render(f, content_area, states, store),
        Tab::Dependencies => dependencies::render(f, content_area, states, store),
        Tab::Files => files::render(f, content_area, states, store),
        Tab::Pre => (),
        Tab::Run => run::render(f, content_area, states, store),
        Tab::Cancel => cancel::render(f, content_area, states, store),
        Tab::Post => (),
        Tab::Chat => chat::render(f, content_area, states, store),
    }
}

pub fn handle_key(key: &event::KeyEvent, states: &mut ui::states::States, store: &mut data_model::Store) {
    use event::KeyCode;

    let states_current = &mut states.job_states.detail;
    if matches!(states_current.tab_action, Tab::Chat) {
        chat::handle_key(key, states, store);
    } else {
        match key.code {
            KeyCode::Char('p') | KeyCode::Char('P') => states_current.tab_action = Tab::Pod,
            KeyCode::Char('s') | KeyCode::Char('S') => states_current.tab_action = Tab::Status,
            KeyCode::Char('d') | KeyCode::Char('D') => states_current.tab_action = Tab::Dependencies,
            KeyCode::Char('f') | KeyCode::Char('F') => states_current.tab_action = Tab::Files,
            KeyCode::Char('e') | KeyCode::Char('E') => states_current.tab_action = Tab::Pre,
            KeyCode::Char('r') | KeyCode::Char('R') => states_current.tab_action = Tab::Run,
            KeyCode::Char('a') | KeyCode::Char('A') => states_current.tab_action = Tab::Cancel,
            KeyCode::Char('t') | KeyCode::Char('T') => states_current.tab_action = Tab::Post,
            KeyCode::Char('c') | KeyCode::Char('C') => states_current.tab_action = Tab::Chat,
            KeyCode::Enter => {
                match match states_current.tab_action {
                    Tab::Pod => pod::action(states, store),
                    Tab::Status => Ok(()),
                    Tab::Dependencies => Ok(()),
                    Tab::Files => Ok(()),
                    Tab::Pre => pre::action(states, store),
                    Tab::Run => run::action(states, store),
                    Tab::Cancel => cancel::action(states, store),
                    Tab::Post => post::action(states, store),
                    Tab::Chat => unreachable!()
                } {
                    Ok(_) => (),
                    Err(e) => states.update_info(format!("job action error: {e}"), MessageLevel::Error),
                }
            }
            KeyCode::Esc => states.job_states.show_page = super::ShowPage::List,
            _ => {
                match states_current.tab_action {
                    Tab::Pod => (),
                    Tab::Status => status::handle_key(key, states, store),
                    Tab::Dependencies => dependencies::handle_key(key, states, store),
                    Tab::Files => files::handle_key(key, states, store),
                    Tab::Pre => (),
                    Tab::Run => (),
                    Tab::Cancel => (),
                    Tab::Post => (),
                    Tab::Chat => (),
                }
            }
        }
    }
}

mod params;
mod pod;
mod status;
mod dependencies;
mod files;
mod pre;
mod run;
mod cancel;
mod post;
mod chat;

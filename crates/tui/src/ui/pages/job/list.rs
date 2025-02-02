use ratatui::prelude::*;
use ratatui::widgets::*;
use crossterm::event;

use crate::data_model;
use crate::ui;
use crate::ui::components;
use crate::ui::layout::info::MessageLevel;

#[derive(Default, PartialEq)]
pub enum Tab {
    #[default]
    New,
    Chat
}

#[derive(Default)]
pub struct States {
    pub tab_action: Tab,
}

pub fn render(f: &mut Frame, area: Rect, states: &mut ui::states::States, store: &data_model::Store) {
    let current_style = states.get_style(true);
    let states_current = &mut states.job_states.list;

    let action_selected = match states_current.tab_action {
        Tab::New => 0,
        Tab::Chat => 1
    };

    let job_mgr  = store.job_mgr.lock().unwrap();
    let jobs_string: Vec<String> = job_mgr.jobs.values()
        .map(|j| j.to_string())
        .collect();

    let job_list = List::new(jobs_string)
        .block(Block::bordered().title(" Jobs "))
        .direction(ListDirection::TopToBottom);

    let top_mid_bottom = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Max(5), Constraint::Min(1)]) 
        .split(area);
    let (top, mid, bottom) = (top_mid_bottom[0], top_mid_bottom[1], top_mid_bottom[2]);

    components::job_list_action_bar::render(f, top, current_style, action_selected);
    components::helper_hint::render_job_new(f, mid, current_style);
    f.render_widget(job_list, bottom);
}

pub fn handle_key(key: &event::KeyEvent, states: &mut ui::states::States, store: &data_model::Store) {
    use event::KeyCode;

    match key.code {
        KeyCode::Char('n') | KeyCode::Char('N') => {
            let states_current = &mut states.job_states.list;
            states_current.tab_action = Tab::New;
        }
        KeyCode::Char('c') | KeyCode::Char('C') => {
            let states_current = &mut states.job_states.list;
            states_current.tab_action = Tab::Chat;
        }
        KeyCode::Enter => {
            let states_current = &mut states.job_states.list;
            match states_current.tab_action {
                Tab::New => if store.project_sel.is_none() {
                    states.info_states.message = ("no project selected".to_string(), MessageLevel::Warn);
                } else {
                    states.job_states.show_page = super::ShowPage::Detail;
                }
                Tab::Chat => states.job_states.show_page = super::ShowPage::Chat,
            }
        }
        _ => ()
    }
}

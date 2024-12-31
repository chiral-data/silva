use crossterm::event;
use ratatui::prelude::*;

use crate::data_model;
use crate::ui;
use crate::ui::layout::info::MessageLevel;

pub fn render(f: &mut Frame, area: Rect, states: &mut ui::states::States, _store: &data_model::Store) {
    let current_style = states.get_style(true);
    ui::components::job_new_helper::render(f, area, current_style);
}

pub fn handle_key(key: &event::KeyEvent, states: &mut ui::states::States, store: &mut data_model::Store) {
    use event::KeyCode;

    if key.code == KeyCode::Enter {
        if store.project_sel.is_none() {
            states.info_states.message = ("no project selected".to_string(), MessageLevel::Warn);
        } else {
            states.tabs_states.tab = ui::layout::tabs::Tab::Job;
            // states.job_states.show_page = ui::pages::job::ShowPage::AppList;
            states.job_states.show_page = ui::pages::job::ShowPage::Detail;
        }
    } 
}

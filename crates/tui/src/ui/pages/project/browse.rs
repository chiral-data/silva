use crossterm::event;
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::data_model;
use crate::ui;
use crate::ui::layout::info::MessageLevel;

#[derive(Default)]
pub struct States {
    list_state_file: ListState,
}

pub fn render(f: &mut Frame, area: Rect, states: &mut ui::states::States, store: &data_model::Store) {
    let current_style = states.get_style(true);
    let states_current = &mut states.project_states.browse;

    if let Some(proj) = store.project_sel.as_ref() {
        ui::components::proj_browser::render(
            f, area, 
            current_style, proj.get_dir(), proj.get_files(), &mut states_current.list_state_file
        );
    } else {
        states.info_states.message = ("no selected project".to_string(), MessageLevel::Warn);
    }
}

pub fn handle_key(key: &event::KeyEvent, states: &mut ui::states::States, store: &mut data_model::Store) {
    let states_current = &mut states.project_states.browse;
    if let Some(proj) = store.project_sel.as_ref() {
        ui::components::proj_browser::handle_key(
            key, 
            proj.get_dir(), proj.get_files(), &mut states_current.list_state_file
        );
    } else {
        states.info_states.message = ("no selected project".to_string(), MessageLevel::Warn);
    }
}

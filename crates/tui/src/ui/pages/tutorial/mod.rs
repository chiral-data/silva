use ratatui::prelude::*;

#[derive(Default)]
pub struct States {
    // tabs: tabs::States,
    // pub list: list::States,
    // pub browse: browse::States,
}

pub fn render(f: &mut ratatui::prelude::Frame, area: ratatui::prelude::Rect, states: &mut crate::ui::states::States, store: &mut crate::data_model::Store) {
    let top_bottom = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)]) 
        .split(area);
    let (top, bottom) = (top_bottom[0], top_bottom[1]);

    let states_current = &states.project_states;
    // tabs::render(f, top, states);
    // match states_current.tabs.tab {
    //     tabs::Tab::List => list::render(f, bottom, states, store),
    //     tabs::Tab::Browse => browse::render(f, bottom, states, store),
    //     tabs::Tab::NewJob => new_job::render(f, bottom, states, store),
    // } 
}

pub fn handle_key(key: &crossterm::event::KeyEvent, states: &mut crate::ui::states::States, store: &mut crate::data_model::Store) {
    // tabs::handle_key(key, states);

    // let states_current = &states.project_states;
    // match states_current.tabs.tab {
    //     tabs::Tab::List => list::handle_key(key, states, store),
    //     tabs::Tab::Browse => browse::handle_key(key, states, store),
    //     tabs::Tab::NewJob => new_job::handle_key(key, states, store),
    // } 
}

mod list;

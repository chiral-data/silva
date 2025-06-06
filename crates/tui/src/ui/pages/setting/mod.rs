use ratatui::prelude::*;

#[derive(Default)]
pub struct States {
    pub tabs: tabs::States,
    pub account: account::States,
    pub registry: docker_registry::States,
}

pub fn render(f: &mut ratatui::prelude::Frame, area: ratatui::prelude::Rect, states: &mut crate::ui::states::States, store: &crate::data_model::Store) {
    let parts = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Length(3), Constraint::Min(1)]) 
        .split(area);
    let (top, bottom) = (parts[0], parts[1]);

    let states_current = &states.setting_states;
    tabs::render(f, top, states);
    match states_current.tabs.tab {
        tabs::Tab::Account => account::render(f, bottom, states, store),
        tabs::Tab::Registry => docker_registry::render(f, bottom, states, store)
    } 
}

pub async fn handle_key(key: &crossterm::event::KeyEvent, states: &mut crate::ui::states::States, store: &mut crate::data_model::Store) {
    tabs::handle_key(key, states);

    let states_current = &mut states.setting_states;
    match states_current.tabs.tab {
        tabs::Tab::Account => account::handle_key(key, states, store).await,
        tabs::Tab::Registry => docker_registry::handle_key(key, states, store)
    } 
}



mod tabs;
mod account;
mod docker_registry;

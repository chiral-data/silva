use std::path::PathBuf;

use ratatui::prelude::*;
use ratatui::widgets::*;

#[derive(Default)]
pub struct States {
    tabs: tabs::States,
    pub list: list::States,
    pub browse: browse::States,
    // pub app_list: app_list::States,
    // pub app_detail: app_detail::States,
    // pub pod_type: pod_type::States,
}

pub fn render(f: &mut ratatui::prelude::Frame, area: ratatui::prelude::Rect, states: &mut crate::ui::states::States, store: &mut crate::data_model::Store) {
    let top_bottom = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)]) 
        .split(area);
    let (top, bottom) = (top_bottom[0], top_bottom[1]);

    let states_current = &states.project_states;
    tabs::render(f, top, states);
    match states_current.tabs.tab {
        tabs::Tab::List => list::render(f, bottom, states, store),
        tabs::Tab::Browse => browse::render(f, bottom, states, store),
        tabs::Tab::NewJob => todo!()
        // ShowPage::AppList => app_list::render(f, bottom, states, store),
        // ShowPage::AppDetail => app_detail::render(f, bottom, states, store),
        // ShowPage::PodType => pod_type::render(f, bottom, states, store),
    } 
}

pub fn handle_key(key: &crossterm::event::KeyEvent, states: &mut crate::ui::states::States, store: &mut crate::data_model::Store) {
    tabs::handle_key(key, states);

    let states_current = &states.project_states;
    match states_current.tabs.tab {
        tabs::Tab::List => list::handle_key(key, states, store),
        tabs::Tab::Browse => browse::handle_key(key, states, store),
        tabs::Tab::NewJob => todo!()
        // ShowPage::AppList => app_list::handle_key(key, states, store),
        // ShowPage::AppDetail => app_detail::handle_key(key, states, store),
        // ShowPage::PodType => pod_type::handle_key(key, states, store),
    } 
}

mod tabs;
mod list;
mod browse;
// mod app_list;
// mod app_detail;
// mod pod_type;

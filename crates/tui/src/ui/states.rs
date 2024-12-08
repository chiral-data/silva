use std::collections::HashMap;

use ratatui::prelude::*;

use crate::ui;

#[derive(Default)]
pub struct States {
    pub focus: ui::Focus,
    pub tab: ui::tabs::States,
    pub info: ui::info::States,
    pub app: ui::app::States,
    pub resource: ui::resource::States,
    pub project: ui::project::States,
    pub job: ui::job::States,
    pub account: ui::account::States,
    pub handlers: HashMap<usize, tokio::task::JoinHandle<()>>,
}

impl States {
    pub fn get_style(&self, focus: ui::Focus) -> Style {
        if self.focus == focus {
            Style::default().fg(ui::COLOR_FOCUS)
        } else {
            Style::default()
        }
    }
}

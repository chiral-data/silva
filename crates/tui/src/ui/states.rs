use ratatui::prelude::*;

use crate::data_model;
use super::layout::*;
use super::pages::*;

const COLOR_FOCUS: style::Color = style::Color::Yellow;

#[derive(Default)]
pub struct States {
    pub tabs_states: tabs::States,
    pub info_states: info::States,
    // pub tutorial_states: tutorial::States,
    pub project_states: project::States,
    pub job_states: job::States,
    pub setting_states: setting::States,
    // pub handlers: HashMap<usize, tokio::task::JoinHandle<()>>,
}

impl States {
    pub fn initialize(&mut self, store: &data_model::Store) {
        // select one account by default
        if let Some(acc_id_selected) = store.setting_mgr.account_id_sel.as_ref() {
            if let Some(idx) = store.account_mgr.accounts.iter().position(|acc| acc.id() == acc_id_selected) {
                self.setting_states.account.list.select(Some(idx));
            }
        }
    }

    pub fn get_style(&self, is_focus: bool) -> Style {
        if is_focus {
            Style::default().fg(COLOR_FOCUS)
        } else {
            Style::default()
        }
    }

    pub fn update_info(&mut self, m: String, l: info::MessageLevel) {
        self.info_states.message = (m, l);
    }
}

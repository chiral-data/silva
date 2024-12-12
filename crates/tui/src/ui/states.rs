use ratatui::prelude::*;

use crate::{data_model, ui};

#[derive(Default)]
pub struct States {
    pub focus: ui::Focus,
    pub tab: ui::tabs::States,
    pub info: ui::info::States,
    pub infra: ui::infra::States,
    pub project: ui::project::States,
    pub job: ui::job::States,
    pub setting: ui::setting::States,
    // pub handlers: HashMap<usize, tokio::task::JoinHandle<()>>,
}

impl States {
    pub fn initialize(&mut self, store: &data_model::Store) {
        if let Some(acc_id_selected) = store.setting_mgr.account_id_sel.as_ref() {
            if let Some(idx) = store.account_mgr.accounts.iter().position(|acc| acc.id() == acc_id_selected) {
                self.setting.account.list.select(Some(idx));
            }
        }
    }

    pub fn get_style(&self, focus: ui::Focus) -> Style {
        if self.focus == focus {
            Style::default().fg(ui::COLOR_FOCUS)
        } else {
            Style::default()
        }
    }
}

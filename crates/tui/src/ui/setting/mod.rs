#[derive(Default)]
pub enum ShowPage {
    #[default]
    Account,
}

#[derive(Default)]
pub struct States {
    pub show_page: ShowPage,
    pub account: account::States,
}

pub fn render(f: &mut ratatui::prelude::Frame, area: ratatui::prelude::Rect, states: &mut crate::ui::States, store: &crate::data_model::Store) {
    match states.setting.show_page {
        ShowPage::Account => account::render(f, area, states, store),
    } 
}

pub fn handle_key(key: &crossterm::event::KeyEvent, states: &mut crate::ui::States, store: &crate::data_model::Store) {
    match states.setting.show_page {
        ShowPage::Account => account::handle_key(key, states, store) 
    } 
}




mod account;

mod list;

#[derive(Default)]
pub struct States {
    pub list: list::States,
}

pub fn render(f: &mut ratatui::prelude::Frame, area: ratatui::prelude::Rect, states: &mut crate::ui::States, store: &crate::data_model::Store) {
    list::render(f, area, states, store);
}

pub fn handle_key(key: &crossterm::event::KeyEvent, states: &mut crate::ui::States, store: &mut crate::data_model::Store) {
    list::handle_key(key, states, store);
}

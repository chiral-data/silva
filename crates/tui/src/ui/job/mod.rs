
#[derive(Default)]
pub enum ShowPage {
    #[default]
    List,
    Detail,
}

#[derive(Default)]
pub struct States {
    pub show_page: ShowPage,
    pub list: list::States,
}

pub fn render(f: &mut ratatui::prelude::Frame, area: ratatui::prelude::Rect, states: &mut crate::ui::States, store: &crate::data_model::Store) {
    match states.job.show_page {
        ShowPage::List => list::render(f, area, states, store),
        ShowPage::Detail => detail::render(f, area, states, store),
    } 
}

pub fn handle_key(key: &crossterm::event::KeyEvent, states: &mut crate::ui::States, store: &crate::data_model::Store) {
    match states.job.show_page {
        ShowPage::List => list::handle_key(key, states, store),
        ShowPage::Detail => detail::handle_key(key, states, store)
    } 
}



mod list;
mod detail;

mod list;
mod pod_type;

#[derive(Default)]
pub enum ShowPage {
    #[default]
    List,
    PodType,
}

#[derive(Default)]
pub struct States {
    pub show_page: ShowPage,
    pub pod_type: pod_type::States,
}

pub fn render(f: &mut ratatui::prelude::Frame, area: ratatui::prelude::Rect, states: &mut crate::ui::States, store: &mut crate::data_model::Store) {
    match states.resource.show_page {
        ShowPage::List => list::render(f, area, states),
        ShowPage::PodType => pod_type::render(f, area, states, store),

    } 
}

pub fn handle_key(key: &crossterm::event::KeyEvent, states: &mut crate::ui::States, store: &mut crate::data_model::Store) {
    match states.resource.show_page {
        ShowPage::List => todo!(), // list::handle_key(key, states),
        ShowPage::PodType => pod_type::handle_key(key, states, store) 
    } 
}

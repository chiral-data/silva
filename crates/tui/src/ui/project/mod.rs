#[derive(Default)]
enum ShowPage {
    #[default]
    ProjectList,
    AppList,
    AppDetail,
    PodType,
}

#[derive(Default)]
pub struct States {
    show_page: ShowPage,
    pub proj_list: proj_list::States,
    pub app_list: app_list::States,
    pub app_detail: app_detail::States,
    pub pod_type: pod_type::States,
}

pub fn render(f: &mut ratatui::prelude::Frame, area: ratatui::prelude::Rect, states: &mut crate::ui::States, store: &mut crate::data_model::Store) {
    match states.project.show_page {
        ShowPage::ProjectList => proj_list::render(f, area, states, store),
        ShowPage::AppList => app_list::render(f, area, states, store),
        ShowPage::AppDetail => app_detail::render(f, area, states, store),
        ShowPage::PodType => pod_type::render(f, area, states, store),
    } 
}

pub fn handle_key(key: &crossterm::event::KeyEvent, states: &mut crate::ui::States, store: &mut crate::data_model::Store) {
    match states.project.show_page {
        ShowPage::ProjectList => proj_list::handle_key(key, states, store),
        ShowPage::AppList => app_list::handle_key(key, states, store),
        ShowPage::AppDetail => app_detail::handle_key(key, states, store),
        ShowPage::PodType => pod_type::handle_key(key, states, store),
    } 
}


mod proj_list;
mod app_list;
mod app_detail;
mod pod_type;

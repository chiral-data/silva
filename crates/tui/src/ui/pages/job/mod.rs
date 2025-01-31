
#[derive(Default)]
pub enum ShowPage {
    #[default]
    List,
    Detail,
    AppList,
    AppDetail,
    PodType,
    Chat,
}

#[derive(Default)]
pub struct States {
    pub show_page: ShowPage,
    pub list: list::States,
    pub detail: detail::States,
    // for cloud resource selection
    pub app_list: app_list::States,
    pub app_detail: app_detail::States,
    pub pod_type: pod_type::States,
    pub chat: chat::States,
}

pub fn render(f: &mut ratatui::prelude::Frame, area: ratatui::prelude::Rect, states: &mut crate::ui::states::States, store: &mut crate::data_model::Store) {
    match states.job_states.show_page {
        ShowPage::List => list::render(f, area, states, store),
        ShowPage::Detail => detail::render(f, area, states, store),
        ShowPage::AppList => app_list::render(f, area, states, store),
        ShowPage::AppDetail => app_detail::render(f, area, states, store),
        ShowPage::PodType => pod_type::render(f, area, states, store),
        ShowPage::Chat => chat::render(f, area, states, store),
    } 
}

pub fn handle_key(key: &crossterm::event::KeyEvent, states: &mut crate::ui::states::States, store: &mut crate::data_model::Store) {
    match states.job_states.show_page {
        ShowPage::List => list::handle_key(key, states, store),
        ShowPage::Detail => detail::handle_key(key, states, store),
        ShowPage::AppList => app_list::handle_key(key, states, store),
        ShowPage::AppDetail => app_detail::handle_key(key, states, store),
        ShowPage::PodType => pod_type::handle_key(key, states, store),
        ShowPage::Chat => chat::handle_key(key, states, store),
    } 
}



mod list;
mod detail;
// for cloud resource selection
mod app_list;
mod app_detail;
mod pod_type;
mod chat;

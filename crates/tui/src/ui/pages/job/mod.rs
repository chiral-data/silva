
#[derive(Default)]
pub enum ShowPage {
    #[default]
    List,
    Detail,
    AppList,
    AppDetail,
    PodType,
    ConfigSelect,
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
    pub config_select: config_select::States,
    pub selected_job_id: Option<usize>,
}

impl States {
    pub fn get_selected_job_id(&self) -> Option<usize> {
        self.selected_job_id
    }

    pub fn set_selected_job_id(&mut self, job_id: Option<usize>) {
        self.selected_job_id = job_id;
    }

    pub fn get_current_job_id(&self) -> usize {
        self.selected_job_id.unwrap_or(0)
    }
}

pub fn render(f: &mut ratatui::prelude::Frame, area: ratatui::prelude::Rect, states: &mut crate::ui::states::States, store: &mut crate::data_model::Store) {
    match states.job_states.show_page {
        ShowPage::List => list::render(f, area, states, store),
        ShowPage::Detail => detail::render(f, area, states, store),
        ShowPage::AppList => app_list::render(f, area, states, store),
        ShowPage::AppDetail => app_detail::render(f, area, states, store),
        ShowPage::PodType => pod_type::render(f, area, states, store),
        ShowPage::ConfigSelect => config_select::render(f, area, states, store),
    } 
}

pub fn handle_key(key: &crossterm::event::KeyEvent, states: &mut crate::ui::states::States, store: &mut crate::data_model::Store) {
    match states.job_states.show_page {
        ShowPage::List => list::handle_key(key, states, store),
        ShowPage::Detail => detail::handle_key(key, states, store),
        ShowPage::AppList => app_list::handle_key(key, states, store),
        ShowPage::AppDetail => app_detail::handle_key(key, states, store),
        ShowPage::PodType => pod_type::handle_key(key, states, store),
        ShowPage::ConfigSelect => config_select::handle_key(key, states, store),
    } 
}



mod list;
mod detail;
// for cloud resource selection
mod app_list;
mod app_detail;
mod pod_type;
mod config_select;

use ratatui::prelude::*;
use ratatui::widgets::*;
use crossterm::event;

use crate::data_model;
use crate::ui;

#[derive(Default)]
pub struct States {
    pub list: ListState
}

pub fn render(f: &mut Frame, area: Rect, states: &mut ui::states::States, store: &data_model::Store) {
    let current_style = states.get_style(true);
    let states_current = &mut states.job_states.app_list;

    if states_current.list.selected().is_none() {
        states_current.list.select(Some(0));
    }

    let items: Vec<String> = store.app_mgr.apps
        .iter().map(|app| format!("{:15} [{}]", app.as_str(), app.keywords()))
        .collect();

    let app_list = List::new(items)
        .block(Block::bordered().title("Available Applications"))
        .highlight_style(Style::new().reversed())
        .highlight_symbol(">>[Enter] ")
        .repeat_highlight_symbol(true)
        .style(current_style)
        .direction(ListDirection::TopToBottom);

    f.render_stateful_widget(app_list, area, &mut states_current.list);
}

pub fn handle_key(key: &event::KeyEvent, states: &mut ui::states::States, store: &data_model::Store) {
    use event::KeyCode;

    let states_current = &mut states.job_states.app_list;
    match key.code {
        KeyCode::Enter => {
            if let Some(idx_sel) = states_current.list.selected() {
                states.job_states.show_page = super::ShowPage::AppDetail;
                let app_sel = store.app_mgr.apps.get(idx_sel).unwrap();
                if states.job_states.app_detail.app != *app_sel {
                    states.job_states.app_detail.pod_types = store.pod_type_mgr.for_applications.get(app_sel)
                        .map(|sp_ids| sp_ids
                            .iter()
                            .map(|id| store.pod_type_mgr.pod_types.get(id).unwrap().to_owned())
                            .collect()
                        )
                        .unwrap_or_default();
                    states.job_states.app_detail.app.clone_from(app_sel);
                }
            }
        }
        KeyCode::Esc => states.job_states.show_page = super::ShowPage::List,
        KeyCode::Up => {
            let total = store.app_mgr.apps.len(); 
            let mut sel_idx = states_current.list.selected().unwrap_or(0);
            sel_idx = (sel_idx + total - 1) % total; 
            states_current.list.select(Some(sel_idx));
        }
        KeyCode::Down => {
            let mut sel_idx = states_current.list.selected().unwrap_or(0);
            sel_idx = (sel_idx + 1) % store.app_mgr.apps.len();
            states_current.list.select(Some(sel_idx));
        }
        _ => ()
    }
}

pub fn get_selected<'a>(states: &ui::states::States, store: &'a data_model::Store) -> Option<&'a data_model::app::App> {
    states.job_states.app_list.list.selected()
        .map(|index| store.app_mgr.apps.get(index))?
}

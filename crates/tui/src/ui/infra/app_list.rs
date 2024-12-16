use ratatui::prelude::*;
use ratatui::widgets::*;
use crossterm::event;

use crate::data_model;
use crate::ui;

#[derive(Default)]
pub struct States {
    list: ListState
}

pub fn render(f: &mut Frame, area: Rect, states: &mut ui::States, store: &data_model::Store) {
    let current_style = states.get_style(ui::Focus::Main);
    let states_current = &mut states.infra.app_list;

    if states_current.list.selected().is_none() {
        states_current.list.select(Some(0));
    }

    let items: Vec<&str> = store.app_mgr.apps
        .iter().map(|app| app.as_str())
        .collect();

    let list = List::new(items)
        .block(Block::bordered().title("Available Applications"))
        .highlight_style(Style::new().reversed())
        .highlight_symbol(">> ")
        .repeat_highlight_symbol(true)
        .style(current_style)
        .direction(ListDirection::TopToBottom);

    f.render_stateful_widget(list, area, &mut states_current.list);
}

pub fn handle_key(key: &event::KeyEvent, states: &mut ui::States, store: &data_model::Store) {
    use event::KeyCode;

    let states_current = &mut states.infra.app_list;
    match key.code {
        KeyCode::Enter => {
            if let Some(idx_sel) = states_current.list.selected() {
                states.infra.show_page = super::ShowPage::AppDetail;
                let app_sel = store.app_mgr.apps.get(idx_sel).unwrap();
                if states.infra.app_detail.app != *app_sel {
                    states.infra.app_detail.pod_types = store.pod_type_mgr.for_applications.get(app_sel)
                        .map(|sp_ids| sp_ids
                            .iter()
                            .map(|id| store.pod_type_mgr.pod_types.get(id).unwrap().to_owned())
                            .collect()
                        )
                        .unwrap_or_default();
                    states.infra.app_detail.app.clone_from(app_sel);
                }
            }
        }
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

pub fn get_selected<'a>(states: &ui::States, store: &'a data_model::Store) -> Option<&'a data_model::app::App> {
    states.infra.app_list.list.selected()
        .map(|index| store.app_mgr.apps.get(index))?
}

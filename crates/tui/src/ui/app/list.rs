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
    let states = &mut states.app.list;
    if states.list.selected().is_none() {
        states.list.select(Some(0));
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

    f.render_stateful_widget(list, area, &mut states.list);
}

pub fn handle_key(key: &event::KeyEvent, states: &mut ui::States, store: &data_model::Store) {
    use event::KeyCode;

    match key.code {
        KeyCode::Enter => {
            if let Some(idx_sel) = states.app.list.list.selected() {
                states.app.show_page = super::ShowPage::Detail;
                let app_sel = store.app_mgr.apps.get(idx_sel).unwrap();
                if states.app.detail.app != *app_sel {
                    states.app.detail.pod_types = store.pod_type_mgr.for_applications.get(app_sel)
                        .map(|sp_ids| sp_ids
                            .iter()
                            .map(|id| store.pod_type_mgr.pod_types.get(id).unwrap().to_owned())
                            .collect()
                        )
                        .unwrap_or_default();
                    states.app.detail.app.clone_from(app_sel);
                }

            }
        }
        KeyCode::Up => {
            let total = store.app_mgr.apps.len(); 
            let mut sel_idx = states.app.list.list.selected().unwrap_or(0);
            sel_idx = (sel_idx + total - 1) % total; 
            states.app.list.list.select(Some(sel_idx));
        }
        KeyCode::Down => {
            let mut sel_idx = states.app.list.list.selected().unwrap_or(0);
            sel_idx = (sel_idx + 1) % store.app_mgr.apps.len();
            states.app.list.list.select(Some(sel_idx));
        }
        _ => ()
    }
}

pub fn get_selected<'a>(states: &ui::States, store: &'a data_model::Store) -> Option<&'a data_model::app::App> {
    states.app.list.list.selected()
        .map(|index| store.app_mgr.apps.get(index))?
}

use ratatui::prelude::*;
use ratatui::widgets::*;
use crossterm::event;

use crate::data_model;
use crate::ui;

#[derive(Default)]
pub struct States {
    pub list: ListState
}

pub fn render(f: &mut Frame, area: Rect, states: &mut ui::States, store: &data_model::Store) {
    let current_style = states.get_style(true);
    let states_current = &mut states.setting.registry;
    if states_current.list.selected().is_none() {
        states_current.list.select(Some(0));
    }

    let registry_strings: Vec<String> = store.registry_mgr.registries.iter()
        .map(|r| format!("{}{r}", if Some(r.id()) == store.setting_mgr.registry_id_sel {
                "* "
            } else { "  " })
        )
        .collect();
    let list = List::new(registry_strings)
        .block(Block::bordered().title(" Select Registry "))
        .style(Style::new().white())
        .highlight_style(Style::new().reversed())
        .highlight_symbol(">>[Space] ")
        .repeat_highlight_symbol(true)
        .style(current_style)
        .direction(ListDirection::TopToBottom);

    f.render_stateful_widget(list, area, &mut states_current.list);
}

pub fn handle_key(key: &event::KeyEvent, states: &mut ui::States, store: &mut data_model::Store) {
    use event::KeyCode;

    let states_current = &mut states.setting.registry;
    match key.code {
        KeyCode::Up => {
            let total = store.registry_mgr.registries.len();
            if total > 0 {
                let mut sel_idx = states_current.list.selected().unwrap_or(0);
                sel_idx = (sel_idx + total - 1) % total;
                states_current.list.select(Some(sel_idx));
            }
        }
        KeyCode::Down => {
            let total = store.registry_mgr.registries.len();
            if total > 0 {
                let mut sel_idx = states_current.list.selected().unwrap_or(0);
                sel_idx = (sel_idx + 1) % total; 
                states_current.list.select(Some(sel_idx));
            }
        }
        KeyCode::Char(' ') => {
            let sel_idx = states_current.list.selected().unwrap_or(0);
            let registry_sel = store.registry_mgr.registries.get(sel_idx).unwrap();
            store.setting_mgr.registry_id_sel = Some(registry_sel.id().to_string());
            store.setting_mgr.save().unwrap();
        }
        _ => ()
    }
}

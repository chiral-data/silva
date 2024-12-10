use ratatui::prelude::*;
use ratatui::widgets::*;
use crossterm::event;

use crate::constants;
use crate::data_model;
use crate::ui;

#[derive(Default)]
pub struct States {
    pub list: ListState
}

pub fn render(f: &mut Frame, area: Rect, states: &mut ui::States, store: &data_model::Store) {
    let current_style = states.get_style(ui::Focus::Main);
    let states_current = &mut states.setting.account;
    if states_current.list.selected().is_none() {
        states_current.list.select(Some(0));
    }
    let account_strings: Vec<String> = store.account_mgr.get_accounts().iter()
        .map(|a| a.to_string())
        .collect();
    let list = List::new(account_strings)
        .block(Block::bordered().title(" Select Cloud Account "))
        .style(Style::new().white())
        .highlight_style(Style::new().reversed())
        .highlight_symbol(">> ")
        .repeat_highlight_symbol(true)
        .style(current_style)
        .direction(ListDirection::TopToBottom);

    f.render_stateful_widget(list, area, &mut states_current.list);
}

pub fn handle_key(key: &event::KeyEvent, states: &mut ui::States, store: &data_model::Store) {
    use event::KeyCode;

    let states_current = &mut states.setting.account;
    match key.code {
        KeyCode::Up => {
            let total = store.account_mgr.get_accounts().len();
            let mut sel_idx = states_current.list.selected().unwrap_or(0);
            sel_idx = (sel_idx + total - 1) % total;
            states_current.list.select(Some(sel_idx));
        }
        KeyCode::Down => {
            let mut sel_idx = states_current.list.selected().unwrap_or(0);
            sel_idx = (sel_idx + 1) % store.account_mgr.get_accounts().len();
            states_current.list.select(Some(sel_idx));
        }
        _ => ()
    }
}

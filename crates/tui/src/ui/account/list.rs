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
    if store.ac_mgr.get_accounts().is_empty() {
        let xdg_dirs = xdg::BaseDirectories::with_prefix(constants::APP_NAME).unwrap();
        let text: Vec<Line> = vec![
            "Account for cloud services NOT found".to_string(),
            format!("Add account information into {}", xdg_dirs.get_data_home().join(constants::FILE_ACCOUNTS).to_str().unwrap())
        ].into_iter()
        .map(|s| Line::from(s).red())
        .collect();
        let paragraph = Paragraph::new(text)
            .block(Block::bordered().title(" Cloud Account Empty "))
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true });
        f.render_widget(paragraph, area);
    } else {
        let states = &mut states.account.list;
        if states.list.selected().is_none() {
            states.list.select(Some(0));
        }
        let account_strings: Vec<String> = store.ac_mgr.get_accounts().iter()
            .map(|a| a.to_string())
            .collect();
        let list = List::new(account_strings)
            .block(Block::bordered().title(" Your Cloud Accounts "))
            .style(Style::new().white())
            .highlight_style(Style::new().reversed())
            .highlight_symbol(">> ")
            .repeat_highlight_symbol(true)
            .style(current_style)
            .direction(ListDirection::TopToBottom);

        f.render_stateful_widget(list, area, &mut states.list);
    }
}

pub fn handle_key(key: &event::KeyEvent, states: &mut ui::States, store: &data_model::Store) {
    use event::KeyCode;

    match key.code {
        KeyCode::Up => {
            let total = store.ac_mgr.get_accounts().len();
            let mut sel_idx = states.account.list.list.selected().unwrap_or(0);
            sel_idx = (sel_idx + total - 1) % total;
            states.account.list.list.select(Some(sel_idx));
        }
        KeyCode::Down => {
            let mut sel_idx = states.account.list.list.selected().unwrap_or(0);
            sel_idx = (sel_idx + 1) % store.ac_mgr.get_accounts().len();
            states.account.list.list.select(Some(sel_idx));
        }
        _ => ()
    }
}

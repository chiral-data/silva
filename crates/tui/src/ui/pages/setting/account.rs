use ratatui::prelude::*;
use ratatui::widgets::*;
use crossterm::event;

use crate::constants;
use crate::data_model;
use crate::ui;
use crate::utils;

#[derive(Default)]
pub struct States {
    pub list: ListState
}

pub fn render(f: &mut Frame, area: Rect, states: &mut ui::states::States, store: &data_model::Store) {
    let current_style = states.get_style(true);

    if store.account_mgr.accounts.is_empty() {
        // let xdg_dirs = xdg::BaseDirectories::with_prefix(constants::APP_NAME).unwrap();
        // let data_dir = xdg_dirs.get_data_home();
        let data_dir = utils::file::get_data_dir();
        let filepath_hint = format!("Add account information into {}", data_dir.join(constants::FILENAME_ACCOUNTS).to_str().unwrap());
        let tmp_filepath_hint = data_dir.join(format!("{}.tmp", constants::FILENAME_ACCOUNTS));
        let error_hints: Vec<Line> = vec![
            "Account file for cloud services NOT found",
            filepath_hint.as_str(),
            "",
            "A temporary account file has been created to start from",
            "fill in the cloud API tokens etc",
            "*****",
            tmp_filepath_hint.to_str().unwrap(),
            "[[accounts]]",
            "Sakura.name = \"\"",
            "Sakura.resource_id = \"\"",
            "Sakura.access_token = \"\"",
            "Sakura.access_token_secret = \"\"",
            "*****",
        ].into_iter()
        .map(|s| Line::from(s).red())
        .collect();
        let paragraph = Paragraph::new(error_hints)
            .block(Block::bordered().title(" Cloud Account Empty "))
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true });
        f.render_widget(paragraph, area);
    } else {
        let states_current = &mut states.setting_states.account;
        if states_current.list.selected().is_none() {
            states_current.list.select(Some(0));
        }
        let account_strings: Vec<String> = store.account_mgr.accounts.iter()
            .map(|a| format!("{}{a}", if Some(a.id()) == store.setting_mgr.account_id_sel.as_deref() {
                    "* "
                } else { "  " })
            )
            .collect();
        let list = List::new(account_strings)
            .block(Block::bordered().title(" Select Cloud Account "))
            .style(Style::new().white())
            .highlight_style(Style::new().reversed())
            .highlight_symbol(">>[Enter] ")
            .repeat_highlight_symbol(true)
            .style(current_style)
            .direction(ListDirection::TopToBottom);

        f.render_stateful_widget(list, area, &mut states_current.list);
    }
}

pub async fn handle_key(key: &event::KeyEvent, states: &mut ui::states::States, store: &mut data_model::Store) {
    use event::KeyCode;

    let states_current = &mut states.setting_states.account;
    match key.code {
        KeyCode::Up => {
            let total = store.account_mgr.accounts.len();
            let mut sel_idx = states_current.list.selected().unwrap_or(0);
            sel_idx = (sel_idx + total - 1) % total;
            states_current.list.select(Some(sel_idx));
        }
        KeyCode::Down => {
            let mut sel_idx = states_current.list.selected().unwrap_or(0);
            sel_idx = (sel_idx + 1) % store.account_mgr.accounts.len();
            states_current.list.select(Some(sel_idx));
        }
        KeyCode::Enter => {
            let sel_idx = states_current.list.selected().unwrap_or(0);
            let account_sel = store.account_mgr.accounts.get(sel_idx).unwrap();
            store.setting_mgr.account_id_sel = Some(account_sel.id().to_string());
            store.setting_mgr.save().unwrap();
            store.registry_mgr.initialze(&store.account_mgr, &store.setting_mgr).await.unwrap();
        }
        _ => ()
    }
}

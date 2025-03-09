use std::path::PathBuf;

use crossterm::event;
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::data_model;
use crate::ui;
use crate::utils;

#[derive(Default)]
pub struct States {
    list: ListState,
    tutorial_dirs: Vec<PathBuf>, 
}

pub fn render(f: &mut Frame, area: Rect, states: &mut ui::states::States, _store: &data_model::Store) {
    let current_style = states.get_style(true);
    let states_current = &mut states.tutorial_states.list;
    if states_current.list.selected().is_none() {
        states_current.list.select(Some(0));
    }

    if states_current.tutorial_dirs.is_empty() {
        let dirs_tutorial = utils::dirs::get_tutorial_dirs(); 
        for dir in dirs_tutorial.iter() {
            assert!(dir.is_dir());
        }
        states_current.tutorial_dirs = dirs_tutorial;
    }

    let list = List::new(states_current.tutorial_dirs.iter().map(|path| path.to_str().unwrap()))
        .block(Block::bordered().title(" List of Tutorials "))
        .highlight_style(Style::new().reversed())
        .highlight_symbol(">>[Enter] ")
        .repeat_highlight_symbol(true)
        .style(current_style)
        .direction(ListDirection::TopToBottom);

    f.render_stateful_widget(list, area, &mut states_current.list);
}

pub fn handle_key(key: &event::KeyEvent, states: &mut ui::states::States, _store: &mut data_model::Store) {
    use event::KeyCode;
    let states_current = &mut states.tutorial_states.list;

    match key.code {
        KeyCode::Up => {
            let total = states_current.tutorial_dirs.len();
            if total > 0 {
                let mut sel_idx = states_current.list.selected().unwrap_or(0);
                sel_idx = (sel_idx + total - 1) % total; 
                states_current.list.select(Some(sel_idx));
            }
        }
        KeyCode::Down => {
            let total = states_current.tutorial_dirs.len();
            if total > 0 {
                let mut sel_idx = states_current.list.selected().unwrap_or(0);
                sel_idx = (sel_idx + 1) % total; 
                states_current.list.select(Some(sel_idx));
            }
        }
        KeyCode::Enter => {
            todo!()
            // if let Some(sel_idx) = states_current.list.selected() {
            //     let proj_dir = states_current.proj_dirs.get(sel_idx).unwrap().to_owned();
            //     match store.update_project(&proj_dir) {
            //         Ok(_) => states.project_states.tabs.tab = super::tabs::Tab::Browse,
            //         Err(e) => states.info_states.message = (format!("cannot selecte project{}: {e}", proj_dir.to_str().unwrap()), MessageLevel::Error)
            //     }
            // }
        }
        _ => ()
    }
}

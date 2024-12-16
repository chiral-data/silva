use std::path::PathBuf;

use crossterm::event;
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::data_model;
use crate::envs;
use crate::ui;
use crate::utils;

#[derive(Default)]
pub struct States {
    list: ListState,
    proj_dirs: Vec<PathBuf>, 
}

pub fn render(f: &mut Frame, area: Rect, states: &mut ui::States, _store: &data_model::Store) {
    let current_style = states.get_style(true);
    let states_current = &mut states.project.list;
    if states_current.list.selected().is_none() {
        states_current.list.select(Some(0));
    }

    let dirs_projects = envs::get_projects_home();
    for dir in dirs_projects.iter() {
        assert!(dir.is_dir());
    }
    states_current.proj_dirs = dirs_projects.into_iter()
        .flat_map(utils::file::get_child_dirs)
        .collect();

    let list = List::new(states_current.proj_dirs.iter().map(|path| path.to_str().unwrap()))
        .block(Block::bordered().title("All Projets"))
        .highlight_style(Style::new().reversed())
        .highlight_symbol(">>[Space] ")
        .repeat_highlight_symbol(true)
        .style(current_style)
        .direction(ListDirection::TopToBottom);

    f.render_stateful_widget(list, area, &mut states_current.list);
}

pub fn handle_key(key: &event::KeyEvent, states: &mut ui::States, store: &mut data_model::Store) {
    use event::KeyCode;
    let states_current = &mut states.project.list;

    match key.code {
        KeyCode::Up => {
            let total = states_current.proj_dirs.len();
            if total > 0 {
                let mut sel_idx = states_current.list.selected().unwrap_or(0);
                sel_idx = (sel_idx + total - 1) % total; 
                states_current.list.select(Some(sel_idx));
            }
        }
        KeyCode::Down => {
            let total = states_current.proj_dirs.len();
            if total > 0 {
                let mut sel_idx = states_current.list.selected().unwrap_or(0);
                sel_idx = (sel_idx + 1) % total; 
                states_current.list.select(Some(sel_idx));
            }
        }
        KeyCode::Char(' ')=> {
            if let Some(sel_idx) = states_current.list.selected() {
                store.proj_selected = Some(states_current.proj_dirs.get(sel_idx).unwrap().to_owned());
                match states.job.detail.update(store) {
                    Ok(_) => (),
                    Err(e) => states.info.message = format!("cannot selecte project{}: {e}", store.proj_selected.as_ref().unwrap().to_str().unwrap())
                }
            }
        }
        _ => ()
    }
}

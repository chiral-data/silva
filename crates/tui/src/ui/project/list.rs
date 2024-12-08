use std::fs;
use std::path::PathBuf;

use crossterm::event;
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::data_model;
use crate::envs;
use crate::ui;

#[derive(Default)]
pub struct States {
    list: ListState,
    proj_dirs: Vec<PathBuf>, 
}

pub fn render(f: &mut Frame, area: Rect, states: &mut ui::States, _store: &data_model::Store) {
    let current_style = states.get_style(ui::Focus::Main);
    let states_current = &mut states.project.list;
    if states_current.list.selected().is_none() {
        states_current.list.select(Some(0));
    }

    let dir_projects = envs::get_projects_home();
    assert!(dir_projects.is_dir());
    let entries = fs::read_dir(dir_projects).unwrap();
    states_current.proj_dirs = entries.into_iter()
        .filter_map(|entry| match entry {
            Ok(e) => {
                if e.path().is_dir() {
                    e.path().to_str().map(PathBuf::from)
                } else { None }
            }
            Err(_) => None
        })
        .collect();

    let list = List::new(states_current.proj_dirs.iter().map(|path| path.to_str().unwrap()))
        .block(Block::bordered().title("All Projets"))
        .highlight_style(Style::new().reversed())
        .highlight_symbol(">>[S] ")
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
        KeyCode::Char('s') | KeyCode::Char('S') => {
            if let Some(sel_idx) = states_current.list.selected() {
                store.proj_selected = Some(states_current.proj_dirs.get(sel_idx).unwrap().to_owned());
            }
        }
        _ => ()
    }
}

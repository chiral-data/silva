use std::fs;

use ratatui::prelude::*;
use ratatui::widgets::*;
use crossterm::event;

use crate::data_model;
use crate::ui;
use crate::utils;

pub const HELPER: &[&str] = &[
    "Preview a job", 
    "e.g., generate the docker file and script file for a DOK task for preview", 
];

pub fn render(f: &mut Frame, area: Rect, states: &mut ui::States, _store: &data_model::Store) {
    let current_style = states.get_style(true);
    let states_current = &mut states.job.detail;

    // file list
    let file_list = List::new(states_current.proj_files.iter().map(|s| s.as_str()))
        .style(current_style)
        .highlight_style(Style::new().reversed())
        .block(Block::bordered())
        .direction(ListDirection::TopToBottom);
    if !states_current.proj_files.is_empty() && states_current.list_state_file.selected().is_none() {
        states_current.list_state_file.select(Some(0));
    }

    // file content
    let contents = if states_current.proj_files.is_empty() {
        "".to_string()
    } else {
        let filename = states_current.proj_files.get(states_current.list_state_file.selected().unwrap()).unwrap();
        let filepath = states_current.proj_dir.join(filename);
        if filepath.exists() {
            match fs::read_to_string(&filepath) {
                Ok(s) => s,
                Err(e) => format!("cannot read file {} as string: {e}", filepath.to_str().unwrap())
            }
        } else {
            format!("file {} not exists", filepath.to_str().unwrap())
        }
    };
    let file_contents = Paragraph::new(contents)
        .style(current_style)
        .block(Block::bordered())
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });

    let left_right = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(20),  Constraint::Min(1)]) 
        .split(area);
    let (left, right) = (left_right[0], left_right[1]);
    f.render_stateful_widget(file_list, left, &mut states_current.list_state_file);
    f.render_widget(file_contents, right)
}

pub fn handle_key(key: &event::KeyEvent, states: &mut ui::States, _store: &data_model::Store) {
    use event::KeyCode;

    let states_current = &mut states.job.detail;
    match key.code {
        KeyCode::Up => {
            let total = states_current.proj_files.len(); 
            let mut sel_idx = states_current.list_state_file.selected().unwrap_or(0);
            sel_idx = (sel_idx + total - 1) % total; 
            while states_current.proj_files.get(sel_idx).unwrap().starts_with("---")  {
                sel_idx = (sel_idx + total - 1) % total; 
            }
            states_current.list_state_file.select(Some(sel_idx));
        }
        KeyCode::Down => {
            let total = states_current.proj_files.len(); 
            let mut sel_idx = states_current.list_state_file.selected().unwrap_or(0);
            sel_idx = (sel_idx + 1) % total;
            while states_current.proj_files.get(sel_idx).unwrap().starts_with("---")  {
                sel_idx = (sel_idx + 1) % total;
            }
            states_current.list_state_file.select(Some(sel_idx));
        }
        _ => ()
    }
}

pub fn action(states: &mut ui::States, store: &data_model::Store) -> anyhow::Result<()> {
    let proj_dir = super::params::proj_dir(store)?;
    let job_settings = data_model::job::Job::get_settings(&proj_dir)?;
    states.info.message = "Creating job intermediate files ...".to_string();
    utils::docker::prepare_build_files(&proj_dir, &job_settings)?;
    states.info.message = format!("Preview job done for project {}", proj_dir.to_str().unwrap());
    Ok(())
}

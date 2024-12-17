use std::fs;
use std::path::Path;

use ratatui::prelude::*;
use ratatui::widgets::*;
use crossterm::event;

pub fn render(
    f: &mut Frame, area: Rect, 
    current_style: Style, proj_dir: &Path, proj_files: &[String], list_state_file: &mut ListState,
) {
    // file list
    let file_list = List::new(proj_files.iter().map(|s| s.as_str()))
        .style(current_style)
        .highlight_style(Style::new().reversed())
        .block(Block::bordered())
        .direction(ListDirection::TopToBottom);
    if !proj_files.is_empty() && list_state_file.selected().is_none() {
        list_state_file.select(Some(0));
    }

    // file content
    let contents = if proj_files.is_empty() {
        "".to_string()
    } else {
        let filename = proj_files.get(list_state_file.selected().unwrap()).unwrap();
        let filepath = proj_dir.join(filename);
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
    f.render_stateful_widget(file_list, left, list_state_file);
    f.render_widget(file_contents, right)
}

pub fn handle_key(
    key: &event::KeyEvent, 
    _proj_dir: &Path, proj_files: &[String], list_state_file: &mut ListState,
) {
    use event::KeyCode;

    match key.code {
        KeyCode::Up => {
            let total = proj_files.len(); 
            let mut sel_idx = list_state_file.selected().unwrap_or(0);
            sel_idx = (sel_idx + total - 1) % total; 
            while proj_files.get(sel_idx).unwrap().starts_with('[')  {
                sel_idx = (sel_idx + total - 1) % total; 
            }
            list_state_file.select(Some(sel_idx));
        }
        KeyCode::Down => {
            let total = proj_files.len(); 
            let mut sel_idx = list_state_file.selected().unwrap_or(0);
            sel_idx = (sel_idx + 1) % total;
            while proj_files.get(sel_idx).unwrap().starts_with('[')  {
                sel_idx = (sel_idx + 1) % total;
            }
            list_state_file.select(Some(sel_idx));
        }
        _ => ()
    }
}


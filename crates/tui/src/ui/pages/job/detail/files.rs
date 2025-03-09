use ratatui::prelude::*;
use crossterm::event;

use crate::data_model;
use crate::ui;
use crate::ui::layout::info::MessageLevel;

pub const HELPER: &[&str] = &[
    "Preview job files", 
];

pub fn render(f: &mut Frame, area: Rect, states: &mut ui::states::States, store: &data_model::Store) {
    let current_style = states.get_style(true);
    let states_current = &mut states.job_states.detail;

    // // file list
    // let file_list = List::new(states_current.proj_files.iter().map(|s| s.as_str()))
    //     .style(current_style)
    //     .highlight_style(Style::new().reversed())
    //     .block(Block::bordered())
    //     .direction(ListDirection::TopToBottom);
    // if !states_current.proj_files.is_empty() && states_current.list_state_file.selected().is_none() {
    //     states_current.list_state_file.select(Some(0));
    // }

    // // file content
    // let contents = if states_current.proj_files.is_empty() {
    //     "".to_string()
    // } else {
    //     let filename = states_current.proj_files.get(states_current.list_state_file.selected().unwrap()).unwrap();
    //     let filepath = states_current.proj_dir.join(filename);
    //     if filepath.exists() {
    //         match fs::read_to_string(&filepath) {
    //             Ok(s) => s,
    //             Err(e) => format!("cannot read file {} as string: {e}", filepath.to_str().unwrap())
    //         }
    //     } else {
    //         format!("file {} not exists", filepath.to_str().unwrap())
    //     }
    // };
    // let file_contents = Paragraph::new(contents)
    //     .style(current_style)
    //     .block(Block::bordered())
    //     .alignment(Alignment::Left)
    //     .wrap(Wrap { trim: true });
    //
    //
    if let Some((proj, _)) = store.project_sel.as_ref() {
        // let left_right = Layout::default()
        //     .direction(Direction::Horizontal)
        //     .constraints([Constraint::Length(20),  Constraint::Min(1)]) 
        //     .split(area);
        // let (left, right) = (left_right[0], left_right[1]);
        // f.render_stateful_widget(file_list, left, &mut states_current.list_state_file);
        // f.render_widget(file_contents, right)
        ui::components::proj_browser::render(
            f, area, 
            current_style, proj.get_dir(), proj.get_files().as_ref(), &mut states_current.list_state_file
        );
    } else {
        states.info_states.message = ("no selected project".to_string(), MessageLevel::Warn);
    }
}

pub fn handle_key(key: &event::KeyEvent, states: &mut ui::states::States, store: &data_model::Store) {
    // use event::KeyCode;

    let states_current = &mut states.job_states.detail;
    if let Some((proj, _)) = store.project_sel.as_ref() {
        ui::components::proj_browser::handle_key(
            key, 
            proj.get_dir(), proj.get_files().as_ref(), &mut states_current.list_state_file
        );
    } else {
        states.info_states.message = ("no selected project".to_string(), MessageLevel::Warn);
    }

    // match key.code {
    //     KeyCode::Up => {
    //         let total = states_current.proj_files.len(); 
    //         let mut sel_idx = states_current.list_state_file.selected().unwrap_or(0);
    //         sel_idx = (sel_idx + total - 1) % total; 
    //         while states_current.proj_files.get(sel_idx).unwrap().starts_with("---")  {
    //             sel_idx = (sel_idx + total - 1) % total; 
    //         }
    //         states_current.list_state_file.select(Some(sel_idx));
    //     }
    //     KeyCode::Down => {
    //         let total = states_current.proj_files.len(); 
    //         let mut sel_idx = states_current.list_state_file.selected().unwrap_or(0);
    //         sel_idx = (sel_idx + 1) % total;
    //         while states_current.proj_files.get(sel_idx).unwrap().starts_with("---")  {
    //             sel_idx = (sel_idx + 1) % total;
    //         }
    //         states_current.list_state_file.select(Some(sel_idx));
    //     }
    //     _ => ()
    // }
}


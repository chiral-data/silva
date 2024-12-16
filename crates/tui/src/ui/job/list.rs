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
    let current_style = states.get_style(ui::Focus::Main);

    let text = vec![
        Line::from("Create a new job"),
        Line::from("    based on the selected project"),
        Line::from("    and execute it on the chosen pod"),
    ];
    let paragraph = Paragraph::new(text)
        .block(Block::bordered())
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });

    let actions = vec![
        Line::from("[N]ew job")
    ];
    let action_list = Paragraph::new(actions)
        .block(Block::bordered().title(" Actions "))
        .style(current_style)
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });

    let jobs_string: Vec<String> = store.jl_mgr.jobs.values()
        .map(|j| j.to_string())
        .collect();

    let list = List::new(jobs_string)
        .block(Block::bordered().title(" Jobs "))
        // .highlight_style(Style::new().reversed())
        // .highlight_symbol(">> ")
        // .repeat_highlight_symbol(true)
        .style(current_style)
        .direction(ListDirection::TopToBottom);

    let top_mid_bottom = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Max(5), Constraint::Length(3), Constraint::Min(1)]) 
        .split(area);
    let (top, mid, bottom) = (top_mid_bottom[0], top_mid_bottom[1], top_mid_bottom[2]);

    f.render_widget(paragraph, top);
    f.render_widget(action_list, mid);
    f.render_widget(list, bottom);
}

pub fn handle_key(key: &event::KeyEvent, states: &mut ui::States, _store: &data_model::Store) {
    use event::KeyCode;
    let states_current = &mut states.job.list;
    let states_parent = &mut states.job;

    match key.code {
        // KeyCode::Up => {
        //     let total = states_current.proj_dirs.len();
        //     if total > 0 {
        //         let mut sel_idx = states_current.list.selected().unwrap_or(0);
        //         sel_idx = (sel_idx + total - 1) % total; 
        //         states_current.list.select(Some(sel_idx));
        //     }
        // }
        // KeyCode::Down => {
        //     let total = states_current.proj_dirs.len();
        //     if total > 0 {
        //         let mut sel_idx = states_current.list.selected().unwrap_or(0);
        //         sel_idx = (sel_idx + 1) % total; 
        //         states_current.list.select(Some(sel_idx));
        //     }
        // }
        KeyCode::Char('n') | KeyCode::Char('N') => {
            states_parent.show_page = super::ShowPage::Detail;
        }
        _ => ()
    }
}

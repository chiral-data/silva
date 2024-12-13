use ratatui::prelude::*;
use ratatui::widgets::*;
use crossterm::event;

use crate::data_model;
use crate::ui;

const HELPER_NEW_JOB: &[&str] = &[
    "Create a new job", 
    "based on the selected project", 
    "and execute it on the chosen pod"
];

#[derive(Default)]
pub struct States {
    pub list_state_action: ListState
}

pub fn render(f: &mut Frame, area: Rect, states: &mut ui::States, store: &data_model::Store) {
    let current_style = states.get_style(ui::Focus::Main);
    let states_current = &mut states.job.list;

    let actions = Tabs::new(["[N]ew job"])
        .block(Block::bordered().title(" Actions "))
        .style(current_style);
    if states_current.list_state_action.selected().is_none() {
        states_current.list_state_action.select(Some(0));
    } 
    let helper: Vec<Line> = HELPER_NEW_JOB.iter()
        .map(|&s| Line::from(s))
        .collect();
    let paragraph = Paragraph::new(helper)
        .block(Block::bordered())
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });

    let job_mgr  = store.job_mgr.lock().unwrap();
    let jobs_string: Vec<String> = job_mgr.jobs.values()
        .map(|j| j.to_string())
        .collect();

    let job_list = List::new(jobs_string)
        .block(Block::bordered().title(" Jobs "))
        // .highlight_style(Style::new().reversed())
        // .highlight_symbol(">> ")
        // .repeat_highlight_symbol(true)
        // .style(current_style)
        .direction(ListDirection::TopToBottom);

    let top_mid_bottom = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Max(5), Constraint::Length(3), Constraint::Min(1)]) 
        .split(area);
    let (top, mid, bottom) = (top_mid_bottom[0], top_mid_bottom[1], top_mid_bottom[2]);

    f.render_widget(paragraph, top);
    f.render_widget(actions, mid);
    f.render_widget(job_list, bottom);
}

pub fn handle_key(key: &event::KeyEvent, states: &mut ui::States, _store: &data_model::Store) {
    use event::KeyCode;

    match key.code {
        KeyCode::Char('n') | KeyCode::Char('N') => {
            let states_current = &mut states.job.list;
            states_current.list_state_action.select(Some(0));
        }
        KeyCode::Enter => {
            let states_parent = &mut states.job;
            states_parent.show_page = super::ShowPage::Detail;
        }
        _ => ()
    }
}

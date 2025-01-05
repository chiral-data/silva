use ratatui::prelude::*;
use ratatui::widgets::*;
use crossterm::event;

use crate::data_model;
use crate::ui;

const HELPER_NEW_JOB: &[&str] = &[
    "Create a new job", 
    "based on the selected project and execute it on the chosen pod",
];

#[derive(Default, PartialEq)]
pub enum Tab {
    #[default]
    New
}

#[derive(Default)]
pub struct States {
    tab_action: Tab,
}

pub fn render(f: &mut Frame, area: Rect, states: &mut ui::States, store: &data_model::Store) {
    let current_style = states.get_style(true);
    let states_current = &mut states.job.list;

    let action_selected = match states_current.tab_action {
        Tab::New => 0,
    };
    let tabs_strings: Vec<String> = ["[N]ew"].into_iter()
        .enumerate()
        .map(|(i, s)| format!("{}{s}", if i == action_selected {
            "[Enter] "
        } else { "" }))
        .collect();
    let actions = Tabs::new(tabs_strings)
        .block(Block::bordered().title(" Actions "))
        .select(action_selected)
        .style(current_style);
    let helper_lines: Vec<Line> = HELPER_NEW_JOB.iter()
        .map(|&s| Line::from(s))
        .collect();
    let helper = Paragraph::new(helper_lines)
        .style(current_style)
        .block(Block::bordered())
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });

    let job_mgr  = store.job_mgr.lock().unwrap();
    let jobs_string: Vec<String> = job_mgr.jobs.values()
        .map(|j| j.to_string())
        .collect();

    let job_list = List::new(jobs_string)
        .block(Block::bordered().title(" Jobs "))
        .direction(ListDirection::TopToBottom);

    let top_mid_bottom = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Max(5), Constraint::Min(1)]) 
        .split(area);
    let (top, mid, bottom) = (top_mid_bottom[0], top_mid_bottom[1], top_mid_bottom[2]);

    f.render_widget(actions, top);
    f.render_widget(helper, mid);
    f.render_widget(job_list, bottom);
}

pub fn handle_key(key: &event::KeyEvent, states: &mut ui::States, _store: &data_model::Store) {
    use event::KeyCode;

    match key.code {
        KeyCode::Char('n') | KeyCode::Char('N') => {
            let states_current = &mut states.job.list;
            states_current.tab_action = Tab::New;
        }
        KeyCode::Enter => {
            states.job.show_page = super::ShowPage::Detail;
        }
        _ => ()
    }
}

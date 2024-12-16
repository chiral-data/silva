use ratatui::prelude::*;
use ratatui::widgets::*;
use crossterm::event; 

use crate::ui;

#[derive(Default, PartialEq)]
pub enum Tab {
    #[default]
    List,
    Browse,
    NewJob,
}

#[derive(Default)]
pub struct States {
    pub tab: Tab
}

pub fn render(f: &mut Frame, area: Rect, states: &ui::states::States) {
    let states_current = &states.project_states.tabs;

    let selected = match states_current.tab {
        Tab::List => 0,
        Tab::Browse => 1,
        Tab::NewJob => 2,
    };
    let tabs_strings: Vec<String> = [
            ("List", "[L]ist"), 
            ("Browse", "[B]rowse"),
            ("New Job", "[N]ew Job"),
        ].into_iter()
        .enumerate()
        .map(|(i, s)| if i == selected {
            if i == 2 {
                format!("[Enter] {}", s.0)
            } else { s.0.to_string() }
        } else { s.1.to_string() })
        .collect();
    let tabs = Tabs::new(tabs_strings)
        .block(Block::default().title(" Actions ").borders(Borders::ALL))
        .select(selected)
        .divider("  ");

    f.render_widget(tabs, area);
}

pub fn handle_key(key: &event::KeyEvent, states: &mut ui::states::States) {
    use event::KeyCode;

    let states_current = &mut states.project_states.tabs;
    match key.code {
        KeyCode::Char('l') | KeyCode::Char('L') => states_current.tab = Tab::List, 
        KeyCode::Char('b') | KeyCode::Char('B') => states_current.tab = Tab::Browse, 
        KeyCode::Char('n') | KeyCode::Char('N') => states_current.tab = Tab::NewJob, 
        KeyCode::Enter => {
            if states_current.tab == Tab::NewJob {
                states.tabs_states.tab = ui::layout::tabs::Tab::Job;
            }
        } 
        _ => ()
    }
}

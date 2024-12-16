use ratatui::prelude::*;
use ratatui::widgets::*;
use crossterm::event; 

use crate::ui;

#[derive(Default, PartialEq)]
pub enum Tab {
    #[default]
    Account,
    Registry,
}

#[derive(Default)]
pub struct States {
    pub tab: Tab
}

pub fn render(f: &mut Frame, area: Rect, states: &ui::States) {
    let current_style = states.get_style(true);
    let states_current = &states.setting.tabs;
    let selected = match states_current.tab {
        Tab::Account => 0,
        Tab::Registry => 1,
    };
    let tabs = Tabs::new(vec![
            "[A]ccount",
            "[R]egistry",
        ])
        .block(Block::default().title("").borders(Borders::ALL))
        .select(selected)
        .style(current_style)
        .divider("  ");

    f.render_widget(tabs, area);
}

pub fn handle_key(key: &event::KeyEvent, states: &mut ui::States) {
    use event::KeyCode;

    let states_current = &mut states.setting.tabs;
    match key.code {
        KeyCode::Char('a') | KeyCode::Char('A')=> states_current.tab = Tab::Account, 
        KeyCode::Char('r') | KeyCode::Char('R')=> states_current.tab = Tab::Registry, 
        _ => ()
    }
}

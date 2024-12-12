use ratatui::prelude::*;
use ratatui::widgets::*;
use crossterm::event; 

use crate::ui;

#[derive(Default, PartialEq)]
pub enum Tab {
    #[default]
    Project,
    Infra,
    Job,
    Setting
}

#[derive(Default)]
pub struct States {
    pub tab: Tab
}

pub fn render(f: &mut Frame, area: Rect, states: &ui::States) {
    let current_style = states.get_style(super::Focus::Tab);
    let states = &states.tab;
    let selected = match states.tab {
        Tab::Project => 0,
        Tab::Infra => 1,
        Tab::Job => 2,
        Tab::Setting => 3
    };
    let tabs = Tabs::new(vec![
            "[P]rojects",
            "[I]nfra",
            "[J]obs",
            "[S]ettings"
        ])
        .block(Block::default().title("").borders(Borders::ALL))
        .select(selected)
        .style(current_style)
        .divider("  ");

    f.render_widget(tabs, area);
}

pub fn handle_key(key: &event::KeyEvent, states: &mut ui::States) {
    use event::KeyCode;

    match key.code {
        KeyCode::Char('I') | KeyCode::Char('i') => {
            states.tab.tab = ui::tabs::Tab::Infra;
            states.infra.show_page = ui::infra::ShowPage::AppList;
        }
        KeyCode::Char('P') | KeyCode::Char('p') => states.tab.tab = ui::tabs::Tab::Project,
        KeyCode::Char('J') | KeyCode::Char('j') => states.tab.tab = ui::tabs::Tab::Job,
        KeyCode::Char('S') | KeyCode::Char('s') => states.tab.tab = ui::tabs::Tab::Setting,
        _ => ()
    }
}

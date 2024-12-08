use ratatui::prelude::*;
use ratatui::widgets::*;
use crossterm::event; 

use crate::ui;

#[derive(Default, PartialEq)]
pub enum Tab {
    #[default]
    Project,
    Application,
    Resource,
    Job,
    Account
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
        Tab::Application => 1,
        Tab::Resource => 2,
        Tab::Job => 3,
        Tab::Account => 4
    };
    let tabs = Tabs::new(vec![
            "[P]roject",
            "[A]pplication",
            "[R]esource",
            "[J]ob",
            "[I]nfo"
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
        KeyCode::Char('A') | KeyCode::Char('a') => {
            if states.tab.tab == ui::tabs::Tab::Application {
                states.app.show_page = ui::app::ShowPage::List;
            } else {
                states.tab.tab = ui::tabs::Tab::Application;
            }
        }
        KeyCode::Char('R') | KeyCode::Char('r') => {
            if states.tab.tab == ui::tabs::Tab::Resource {
                states.resource.show_page = ui::resource::ShowPage::List;
            } else {
                states.tab.tab = ui::tabs::Tab::Resource;
            }
        }
        KeyCode::Char('P') | KeyCode::Char('p') => states.tab.tab = ui::tabs::Tab::Project,
        KeyCode::Char('J') | KeyCode::Char('j') => states.tab.tab = ui::tabs::Tab::Job,
        KeyCode::Char('C') | KeyCode::Char('c') => states.tab.tab = ui::tabs::Tab::Account,
        _ => ()
    }
}

use ratatui::prelude::*;
use ratatui::widgets::*;

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
    let current_style = states.get_style(false);
    let states = &states.tab;
    let selected = match states.tab {
        Tab::Project => 0,
        Tab::Infra => 1,
        Tab::Job => 2,
        Tab::Setting => 3
    };
    let tabs = Tabs::new([
            "Projects",
            "Infra",
            "Jobs",
            "Settings"
        ])
        .block(Block::default().title("").borders(Borders::ALL))
        .select(selected)
        .style(current_style)
        .divider("  ");

    f.render_widget(tabs, area);
}


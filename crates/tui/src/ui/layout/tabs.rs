use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::ui;

#[derive(Default, PartialEq)]
pub enum Tab {
    #[default]
    Project,
    Job,
    Setting
}

#[derive(Default)]
pub struct States {
    pub tab: Tab
}

pub fn render(f: &mut Frame, area: Rect, states: &ui::states::States) {
    let current_style = states.get_style(false);
    let states_current = &states.tabs_states;

    let selected = match states_current.tab {
        Tab::Project => 0,
        Tab::Job => 1,
        Tab::Setting => 2
    };
    let tabs = Tabs::new([
            "Projects",
            "Jobs",
            "Settings"
        ])
        .block(Block::default().title("").borders(Borders::ALL))
        .select(selected)
        .style(current_style)
        .divider("  ");

    f.render_widget(tabs, area);
}


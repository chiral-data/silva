use ratatui::prelude::*;
use ratatui::widgets::*;
// use crossterm::event;

use crate::ui;

pub fn render(f: &mut Frame, area: Rect, _states: &mut ui::States) {
    let block = Block::new().title("resource list");

    f.render_widget(block, area);
}



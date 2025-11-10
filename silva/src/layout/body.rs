use ratatui::Frame;

use crate::{
    app::App,
    components::{application, health_check, workflow},
};

pub fn render(frame: &mut Frame, area: ratatui::prelude::Rect, app: &App) {
    match app.selected_tab {
        0 => application::render::render(frame, area, app),
        1 => workflow::render::render(frame, area, app),
        2 => health_check::render::render(frame, area, app),
        _ => {}
    }
}

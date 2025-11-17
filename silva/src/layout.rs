use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
};

use crate::app::App;
use crate::components;

mod body;
pub mod footer;
mod header;
mod sidebar;

pub fn ui(frame: &mut Frame, app: &mut App) {
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(0),    // Main content
            Constraint::Length(3), // Footer
        ])
        .split(frame.area());

    // Header with title and tabs
    header::render(frame, main_layout[0], app);

    // Main content area (full width, no sidebar)
    let content_area = main_layout[1];

    // Main content based on selected tab
    body::render(frame, content_area, app);

    // Footer with status and help
    footer::render(frame, main_layout[2], app);

    // Docker popup (rendered on top if visible)
    if app.workflow_state.show_docker_popup {
        components::docker::render::render(frame, app, frame.area());
    }

    // Params popup (rendered on top if visible)
    if app.workflow_state.show_params_popup {
        if let Some(ref mut params_state) = app.workflow_state.params_editor_state {
            components::workflow::params_editor::render(frame, params_state, frame.area());
        }
    }

    // Help popup (rendered on top if visible)
    if app.show_help {
        sidebar::render(frame, frame.area(), app);
    }
}

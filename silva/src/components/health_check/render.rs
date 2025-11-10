use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph},
};

use super::state::HealthStatus;
use crate::app::App;

pub fn render(frame: &mut Frame, area: ratatui::prelude::Rect, app: &App) {
    // Clear the background
    frame.render_widget(Clear, area);

    // Create the main popup block
    let block = Block::default()
        .title("🏥 System Health Check")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default());

    frame.render_widget(block, area);

    // Inner area for content
    let inner_area = area.inner(ratatui::layout::Margin {
        horizontal: 2,
        vertical: 1,
    });

    // Split into header and content areas
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header with instructions
            Constraint::Min(0),    // Health check results
        ])
        .split(inner_area);

    // Header with instructions
    let header_text = vec![];

    let header = Paragraph::new(header_text)
        .alignment(Alignment::Center)
        .block(Block::default());
    frame.render_widget(header, layout[0]);

    // Health check results
    let mut health_items = Vec::new();

    for check in &app.health_check_state.health_checks {
        let (icon, color) = match check.status {
            HealthStatus::Pass => ("✅", Color::Green),
            HealthStatus::Fail => ("❌", Color::Red),
            HealthStatus::Warning => ("⚠️", Color::Yellow),
            HealthStatus::Checking => ("🔄", Color::Blue),
        };

        health_items.push(ListItem::new(Line::from(vec![
            Span::styled(icon, Style::default().fg(color)),
            Span::styled(
                format!(" {}: ", check.name),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled(check.details.clone(), Style::default().fg(Color::Gray)),
        ])));
    }

    if health_items.is_empty() {
        health_items.push(ListItem::new(Line::from(vec![
            Span::styled("🔄", Style::default().fg(Color::Blue)),
            Span::styled(" Running health checks...", Style::default()),
        ])));
    }

    let health_list = List::new(health_items).block(
        Block::default()
            .title("Results")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded),
    );

    frame.render_widget(health_list, layout[1]);
}

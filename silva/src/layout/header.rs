use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use crate::app::App;

pub fn render(frame: &mut Frame, area: ratatui::prelude::Rect, app: &App) {
    let tabs = ["📦 Applications", "📁 Workflows", "🎛️  Settings"];
    let selected_style = Style::default()
        .fg(crate::style::COLOR_FG)
        .add_modifier(Modifier::BOLD)
        .bg(Color::DarkGray);
    let normal_style = Style::default().fg(Color::White);

    let header_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(12), Constraint::Min(0)])
        .split(area);

    // Title
    let title = Paragraph::new(Line::from(vec![Span::styled(
        "💻 Silva",
        Style::default().add_modifier(Modifier::BOLD),
    )]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded),
    );
    frame.render_widget(title, header_layout[0]);

    // Tabs
    let tab_spans: Vec<Span> = tabs
        .iter()
        .enumerate()
        .map(|(i, &tab)| {
            let style = if i == app.selected_tab {
                selected_style
            } else {
                normal_style
            };
            Span::styled(format!(" {tab} "), style)
        })
        .collect();

    let tabs_paragraph = Paragraph::new(Line::from(tab_spans))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        )
        .alignment(Alignment::Left);
    frame.render_widget(tabs_paragraph, header_layout[1]);
}

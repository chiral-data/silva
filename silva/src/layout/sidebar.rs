use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Padding, Paragraph},
};

use crate::app::App;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    // Calculate popup size (centered, reasonable size)
    let popup_width = 40.min(area.width.saturating_sub(4));
    let popup_height = 20.min(area.height.saturating_sub(4));
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;

    let popup_area = Rect {
        x: popup_x,
        y: popup_y,
        width: popup_width,
        height: popup_height,
    };

    // Clear the area first
    frame.render_widget(Clear, popup_area);
    // Help
    let mut help_text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                format!("{:>12}", "←/→ or h/l "),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Switch tabs"),
        ]),
        Line::from(vec![
            Span::styled(
                format!("{:>12}", "q "),
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::raw("Quit"),
        ]),
    ];

    if app.selected_tab == 0 {
        help_text.push(Line::from(vec![
            Span::styled(
                format!("{:>12}", "↑↓ or j/k "),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Navigate"),
        ]));
        help_text.push(Line::from(vec![
            Span::styled(
                format!("{:>12}", "Enter or d "),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("View Details"),
        ]));
        help_text.push(Line::from(vec![
            Span::styled(
                format!("{:>12}", "Esc or d "),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Close Details"),
        ]));
    } else if app.selected_tab == 1 {
        help_text.push(Line::from(vec![
            Span::styled(
                format!("{:>12}", "↑↓ or j/k "),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Navigate Workflows"),
        ]));
        help_text.push(Line::from(vec![
            Span::styled(
                format!("{:>12}", "r "),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Refresh Workflows"),
        ]));
        help_text.push(Line::from(vec![
            Span::styled(
                format!("{:>12}", "Enter "),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Run Workflow"),
        ]));
        help_text.push(Line::from(vec![
            Span::styled(
                format!("{:>12}", "d "),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Toggle Job Details"),
        ]));
        help_text.push(Line::from(vec![
            Span::styled(
                format!("{:>12}", "p "),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Edit Job Parameters"),
        ]));
        help_text.push(Line::from(vec![
            Span::styled(
                format!("{:>12}", "g "),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Edit Global Parameters"),
        ]));
        help_text.push(Line::from(""));
        help_text.push(Line::from(vec![Span::styled(
            "In Job Details:",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::ITALIC),
        )]));
        help_text.push(Line::from(vec![
            Span::styled(
                format!("{:>12}", "Shift+↑↓ "),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Scroll Logs"),
        ]));
        help_text.push(Line::from(vec![
            Span::styled(
                format!("{:>12}", "PgUp/PgDn "),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Page Scroll"),
        ]));
        help_text.push(Line::from(vec![
            Span::styled(
                format!("{:>12}", "b "),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Scroll to Bottom"),
        ]));
    } else if app.selected_tab == 2 {
        help_text.push(Line::from(vec![
            Span::styled(
                format!("{:>12}", "r "),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Refresh Health Check"),
        ]));
    }

    help_text.push(Line::from(""));
    help_text.push(Line::from(vec![
        Span::styled(
            format!("{:>12}", "i "),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("Toggle Help"),
    ]));

    let help = Paragraph::new(help_text)
        .block(
            Block::default()
                .title(" Help (Press 'i' to close) ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Yellow))
                .padding(Padding::horizontal(1)),
        )
        .alignment(Alignment::Left);
    frame.render_widget(help, popup_area);
}

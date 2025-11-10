use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, Clear, Paragraph, Row, Table, Wrap},
};

use crate::app::App;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    render_applications_list(frame, area, app);

    if app.application_state.show_popup {
        render_popup(frame, area, app);
    }
}

fn render_applications_list(frame: &mut Frame, area: Rect, app: &App) {
    // Applications table
    let rows: Vec<Row> = app
        .application_state
        .catalog
        .applications
        .iter()
        .enumerate()
        .map(|(i, app_item)| {
            let style = if i == app.application_state.selected_index {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
                    .bg(Color::DarkGray)
            } else {
                Style::default()
            };

            let gpu_indicator = if app_item.requirements.gpu {
                "🎮"
            } else {
                "  "
            };

            Row::new(vec![
                Cell::from(app_item.name.clone()),
                Cell::from(app_item.category.clone()),
                Cell::from(app_item.version.clone()),
                Cell::from(app_item.description.clone()),
                Cell::from(gpu_indicator),
            ])
            .style(style)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(20),
            Constraint::Length(18),
            Constraint::Length(15),
            Constraint::Min(30),
            Constraint::Length(4),
        ],
    )
    .header(
        Row::new(vec!["Name", "Category", "Version", "Description", "GPU"]).style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Cyan),
        ),
    )
    .block(
        Block::default()
            .title(format!(
                "Available Applications ({})",
                app.application_state.catalog.applications.len()
            ))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded),
    )
    .column_spacing(2);

    frame.render_widget(table, area);
}

fn render_popup(frame: &mut Frame, area: Rect, app: &App) {
    if let Some(selected_app) = app.application_state.selected_application() {
        // Calculate popup size (70% width, 80% height)
        let popup_width = (area.width as f32 * 0.7) as u16;
        let popup_height = (area.height as f32 * 0.8) as u16;
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

        // Create popup content
        let mut text_lines = vec![
            Line::from(vec![
                Span::styled("Name: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&selected_app.name),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Category: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&selected_app.category),
            ]),
            Line::from(vec![
                Span::styled("Version: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&selected_app.version),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Description:",
                Style::default().add_modifier(Modifier::BOLD),
            )]),
            Line::from(selected_app.long_description.clone()),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Technical Details:",
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(Color::Cyan),
            )]),
            Line::from(vec![
                Span::raw("  Base Image: "),
                Span::styled(&selected_app.base_image, Style::default().fg(Color::Green)),
            ]),
            Line::from(vec![
                Span::raw("  Registry: "),
                Span::styled(
                    selected_app.full_image_name(),
                    Style::default().fg(Color::Green),
                ),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Requirements:",
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(Color::Cyan),
            )]),
            Line::from(vec![
                Span::raw("  GPU Required: "),
                Span::styled(
                    if selected_app.requirements.gpu {
                        "Yes"
                    } else {
                        "No"
                    },
                    Style::default().fg(if selected_app.requirements.gpu {
                        Color::Yellow
                    } else {
                        Color::Green
                    }),
                ),
            ]),
            Line::from(format!(
                "  Memory: {} GB",
                selected_app.requirements.memory_gb
            )),
        ];

        if let Some(cuda) = &selected_app.requirements.cuda_version {
            text_lines.push(Line::from(format!("  CUDA Version: {cuda}")));
        }

        text_lines.push(Line::from(""));
        text_lines.push(Line::from(vec![
            Span::styled("Tags: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(
                selected_app.tags.join(", "),
                Style::default().fg(Color::Magenta),
            ),
        ]));

        text_lines.push(Line::from(""));
        text_lines.push(Line::from(vec![Span::styled(
            "Docker Pull Command:",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Cyan),
        )]));
        text_lines.push(Line::from(vec![Span::styled(
            format!("  {}", selected_app.docker_pull_command()),
            Style::default().fg(Color::Yellow),
        )]));

        text_lines.push(Line::from(""));
        text_lines.push(Line::from(vec![
            Span::styled(
                "Documentation: ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                &selected_app.documentation_url,
                Style::default().fg(Color::Blue),
            ),
        ]));

        let paragraph = Paragraph::new(text_lines)
            .block(
                Block::default()
                    .title(format!(" {} - Details ", selected_app.name))
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Yellow)),
            )
            .wrap(Wrap { trim: true })
            .alignment(Alignment::Left);

        frame.render_widget(paragraph, popup_area);

        // Add instruction at the bottom
        let instruction_area = Rect {
            x: popup_x,
            y: popup_y + popup_height - 1,
            width: popup_width,
            height: 1,
        };

        let instruction = Paragraph::new("Press Esc to close")
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);

        frame.render_widget(instruction, instruction_area);
    }
}

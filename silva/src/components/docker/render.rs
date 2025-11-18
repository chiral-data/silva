use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};
use textwrap::wrap;

use super::{job::JobStatus, logs::LogSource};
use crate::app::App;

/// Renders the Docker logs popup.
pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    // Create centered popup area
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(10),
            Constraint::Percentage(80),
            Constraint::Percentage(10),
        ])
        .split(area);

    let popup_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(10),
            Constraint::Percentage(80),
            Constraint::Percentage(10),
        ])
        .split(popup_layout[1])[1];

    // Clear the background
    f.render_widget(Clear, popup_area);

    // Create the main popup block with background
    let popup_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Black));

    f.render_widget(popup_block, popup_area);

    // Get inner area for content (inside the borders)
    let inner_area = popup_area.inner(ratatui::layout::Margin {
        horizontal: 1,
        vertical: 1,
    });

    let vertical_sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Status
            Constraint::Min(10),   // Jobs list + Logs area
        ])
        .split(inner_area);

    // Render status section
    render_status_section(f, app, vertical_sections[0]);

    // Split the remaining area horizontally for job list and logs
    let horizontal_sections = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30), // Job list
            Constraint::Percentage(70), // Logs
        ])
        .split(vertical_sections[1]);

    // Render job list and logs
    render_job_list_section(f, app, horizontal_sections[0]);
    render_job_logs_section(f, app, horizontal_sections[1]);
}

fn render_status_section(f: &mut Frame, app: &mut App, area: Rect) {
    let docker_state = &app.workflow_state.docker_state;
    let status = if let Some(job) = docker_state.get_selected_job_entry() {
        job.status.clone()
    } else {
        JobStatus::Idle
    };

    let status_color = match status {
        JobStatus::Idle => Color::Gray,
        JobStatus::Completed => Color::Green,
        JobStatus::Failed => Color::Red,
        _ => Color::Yellow,
    };

    // Build status text with workflow info if available
    let mut status_spans = vec![Span::raw("Status: ")];

    status_spans.push(Span::styled(
        status.as_str(),
        Style::default()
            .fg(status_color)
            .add_modifier(Modifier::BOLD),
    ));

    let status_text = vec![Line::from(status_spans)];
    let title = "Docker Job Status";

    let status_paragraph = Paragraph::new(status_text).block(
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    f.render_widget(status_paragraph, area);
}

/// Renders the job list section for multi-job display.
fn render_job_list_section(f: &mut Frame, app: &mut App, area: Rect) {
    let docker_state = &app.workflow_state.docker_state;

    let job_items: Vec<ListItem> = docker_state
        .job_entries
        .iter()
        .enumerate()
        .map(|(idx, job)| {
            let is_selected = docker_state.selected_job_index == Some(idx);
            let (symbol, color) = get_job_status_symbol_and_color(&job.status);

            let mut spans = vec![];

            // Selection indicator
            if is_selected {
                spans.push(Span::styled(
                    "> ",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ));
            } else {
                spans.push(Span::raw("  "));
            }

            // Status symbol
            spans.push(Span::styled(
                symbol,
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::raw(" "));

            // Job name
            let name_style = if is_selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(color)
            };
            spans.push(Span::styled(&job.name, name_style));

            ListItem::new(Line::from(spans))
        })
        .collect();

    let title = format!(
        "Jobs ({}/{})",
        docker_state.jobs.len(),
        docker_state.jobs.len()
    );

    let jobs_list = List::new(job_items).block(
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    f.render_widget(jobs_list, area);
}

/// Renders logs for the currently selected job.
fn render_job_logs_section(f: &mut Frame, app: &mut App, area: Rect) {
    // Store viewport dimensions for scroll calculations
    let docker_state = &mut app.workflow_state.docker_state;
    docker_state.last_viewport_width = area.width as usize;
    docker_state.last_viewport_height = area.height.saturating_sub(2) as usize;

    // Get selected job's logs or show empty message
    let (logs, job_name) = if let Some(job) = docker_state.get_selected_job_entry() {
        (&job.logs, &job.name)
    } else {
        // No job selected, show empty state
        let empty_paragraph = Paragraph::new("No job selected").block(
            Block::default()
                .title("Job Logs")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        );
        f.render_widget(empty_paragraph, area);
        return;
    };

    let log_lines = logs.lines();

    // Calculate available width for log content
    // Account for: borders (2) + timestamp (8) + space (1) + prefix (5) + space (1) = 17 chars
    let prefix_width = 17;
    let available_width = area.width.saturating_sub(2 + prefix_width) as usize;
    let indent = "  "; // Indentation for wrapped continuation lines

    // Create wrapped log items
    let mut log_items: Vec<ListItem> = Vec::new();

    for log_line in log_lines.iter() {
        let source_style = match log_line.source {
            LogSource::Stdout => Style::default().fg(Color::White),
            LogSource::Stderr => Style::default().fg(Color::Red),
        };

        let source_prefix = match log_line.source {
            LogSource::Stdout => "[OUT]",
            LogSource::Stderr => "[ERR]",
        };

        let time_str = log_line.timestamp.format("%H:%M:%S").to_string();

        // Wrap the content if it's too long
        if available_width > 10 {
            let wrapped_lines = wrap(&log_line.content, available_width);

            for (idx, wrapped_line) in wrapped_lines.iter().enumerate() {
                if idx == 0 {
                    // First line with full prefix
                    log_items.push(ListItem::new(Line::from(vec![
                        Span::styled(time_str.clone(), source_style),
                        Span::raw(" "),
                        Span::styled(source_prefix, source_style),
                        Span::raw(" "),
                        Span::styled(wrapped_line.to_string(), source_style),
                    ])));
                } else {
                    // Continuation lines with indentation
                    log_items.push(ListItem::new(Line::from(vec![
                        Span::raw(" ".repeat(time_str.len())),
                        Span::raw(" "),
                        Span::raw(" ".repeat(source_prefix.len())),
                        Span::raw(" "),
                        Span::styled(format!("{indent}{wrapped_line}"), source_style),
                    ])));
                }
            }
        } else {
            // Fallback if area is too small
            log_items.push(ListItem::new(Line::from(vec![
                Span::styled(time_str, source_style),
                Span::raw(" "),
                Span::styled(source_prefix, source_style),
                Span::raw(" "),
                Span::styled(&log_line.content, source_style),
            ])));
        }
    }

    // Apply scrolling after wrapping
    let visible_items: Vec<ListItem> = log_items
        .into_iter()
        .skip(docker_state.scroll_offset)
        .take(area.height.saturating_sub(2) as usize)
        .collect();

    let log_count = log_lines.len();
    let auto_scroll_indicator = if docker_state.auto_scroll_enabled {
        ""
    } else {
        " [AUTO-SCROLL PAUSED]"
    };

    // Get temp folder path for display
    let temp_path_display: String = {
        let temp_path_guard = docker_state.current_temp_workflow_path.lock().unwrap();
        if let Some(ref path) = *temp_path_guard {
            format!(" | Folder: {} | o-Open", path.display())
        } else {
            String::new()
        }
    };

    let title = format!(
        "{} - Logs ({}/{}){}{}",
        job_name,
        docker_state.scroll_offset.min(log_count),
        log_count,
        auto_scroll_indicator,
        temp_path_display
    );

    let logs_list = List::new(visible_items).block(
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    f.render_widget(logs_list, area);
}

/// Returns the visual symbol and color for a job status.
fn get_job_status_symbol_and_color(status: &JobStatus) -> (&'static str, Color) {
    match status {
        JobStatus::Idle => ("○", Color::Gray),
        JobStatus::Pending => ("⬜", Color::Gray),
        JobStatus::PullingImage | JobStatus::BuildingImage | JobStatus::CreatingContainer => {
            ("⟳", Color::Yellow)
        }
        JobStatus::ContainerRunning(_) => ("⟳", Color::Yellow),
        JobStatus::Running => ("⟳", Color::Yellow),
        JobStatus::Completed => ("✓", Color::Green),
        JobStatus::Failed => ("✗", Color::Red),
    }
}

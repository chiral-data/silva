use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::App;

pub fn render(frame: &mut Frame, area: ratatui::prelude::Rect, app: &App) {
    let workflow_state = &app.workflow_state;

    let workflows = workflow_state.workflow_manager.get_workflows();
    let home_path = workflow_state.workflow_manager.home_path();

    let mut items: Vec<ListItem> = Vec::new();

    if workflows.is_empty() {
        // Show empty state message
        items.push(ListItem::new(Line::from(vec![Span::styled(
            "No workflows found",
            Style::default().fg(Color::Yellow),
        )])));
        items.push(ListItem::new(""));
        items.push(ListItem::new(Line::from(vec![Span::styled(
            "Create a new folder in: ",
            Style::default().fg(Color::DarkGray),
        )])));
        items.push(ListItem::new(Line::from(vec![Span::styled(
            format!("  {}", home_path.display()),
            Style::default().fg(Color::Cyan),
        )])));
    } else {
        // Display workflows
        for (idx, workflow) in workflows.iter().enumerate() {
            let is_selected = workflow_state.selected_workflow == Some(idx);

            let mut spans = vec![Span::raw("📁 ")];

            spans.push(Span::styled(
                &workflow.name,
                Style::default().fg(if is_selected {
                    Color::Cyan
                } else {
                    Color::White
                }),
            ));

            spans.push(Span::raw("  "));

            spans.push(Span::styled(
                format!("({})", workflow.created_display()),
                Style::default().fg(Color::DarkGray),
            ));

            let item = if is_selected {
                ListItem::new(Line::from(spans)).style(Style::default().bg(Color::DarkGray))
            } else {
                ListItem::new(Line::from(spans))
            };

            items.push(item);
        }
    }

    let title = if let Some(err) = workflow_state.workflow_manager.last_error() {
        format!("Workflow Folders - Error: {err}")
    } else {
        format!(
            "Workflow Folders ({}) - {}",
            workflows.len(),
            home_path.display()
        )
    };

    let list = List::new(items).block(
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded),
    );

    frame.render_widget(list, area);
}

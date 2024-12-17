use ratatui::prelude::*;
use ratatui::widgets::*;

const HELPER_NEW_JOB: &[&str] = &[
    "Create a new job", 
    "based on the selected project",
];

pub fn render(
    f: &mut Frame, area: Rect, 
    current_style: Style
) {
    let helper_lines: Vec<Line> = HELPER_NEW_JOB.iter()
        .map(|&s| Line::from(s))
        .collect();
    let helper = Paragraph::new(helper_lines)
        .style(current_style)
        .block(Block::bordered())
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });
    f.render_widget(helper, area);
}

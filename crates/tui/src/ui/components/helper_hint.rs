use ratatui::prelude::*;
use ratatui::widgets::*;

fn render(
    f: &mut Frame, area: Rect, 
    current_style: Style, contents: &[&str]
) {
    let helper_lines: Vec<Line> = contents.iter()
        .map(|&s| Line::from(s))
        .collect();
    let helper = Paragraph::new(helper_lines)
        .style(current_style)
        .block(Block::bordered())
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });
    f.render_widget(helper, area);
}

const HELPER_NEW_JOB: &[&str] = &[
    "Create a new job", 
    "based on the selected project",
];

pub fn render_job_new(
    f: &mut Frame, area: Rect, 
    current_style: Style
) {
    render(f, area, current_style, HELPER_NEW_JOB);

}

const HELPER_CHAT: &[&str] = &[
    "Interact with LLM", 
];

pub fn render_chat(
    f: &mut Frame, area: Rect, 
    current_style: Style
) {
    render(f, area, current_style, HELPER_CHAT);

}

use std::time::Instant;

use ratatui::{
    Frame,
    layout::Alignment,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use crate::app::App;

pub struct State {
    pub start_time: Instant,
    pub sys: sysinfo::System,
    pub cpu_usage: f32,
    pub total_memory: f32,
    pub used_memory: f32,
}

impl Default for State {
    fn default() -> Self {
        Self {
            start_time: Instant::now(),
            sys: sysinfo::System::new_all(),
            cpu_usage: 0.0,
            total_memory: 0.0,
            used_memory: 0.0,
        }
    }
}

impl State {
    pub fn update(&mut self) {
        self.sys.refresh_all();
        self.cpu_usage = self.sys.global_cpu_info().cpu_usage();
        self.total_memory = self.sys.total_memory() as f32 / 1e9;
        self.used_memory = self.sys.used_memory() as f32 / 1e9;
    }
}

pub fn render(frame: &mut Frame, area: ratatui::prelude::Rect, app: &App) {
    let state = &app.footer_state;
    let uptime = state.start_time.elapsed().as_secs();
    let footer_text = vec![Line::from(vec![
        Span::styled(
            "Silva: workflow automation in the terminal",
            Style::default().fg(Color::LightYellow),
        ),
        Span::raw(" | "),
        Span::styled("Ver: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("0.1.0"),
        Span::raw(" | "),
        Span::styled("Uptime: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(format!("{uptime}s"), Style::default().fg(Color::Cyan)),
        Span::raw(" | "),
        Span::styled("Memory: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(
            format!("{:.1}Gi / {:.1}Gi", state.used_memory, state.total_memory),
            Style::default().fg(Color::Green),
        ),
        Span::raw(" | "),
        Span::styled("CPU: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(
            format!("{:.1}%", state.cpu_usage),
            Style::default().fg(Color::Yellow),
        ),
    ])];

    let footer = Paragraph::new(footer_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        )
        .alignment(Alignment::Center);
    frame.render_widget(footer, area);
}

use std::{io, time::Duration};

use crossterm::event::{self, Event};
use ratatui::Terminal;

pub mod app;
pub mod components;
pub mod job_config;
mod layout;
mod style;

pub async fn run_app<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>) -> io::Result<()> {
    let mut app = app::App::new().await;
    app.health_check_state.run_health_checks();

    loop {
        app.update().await;
        terminal.draw(|f| layout::ui(f, &mut app))?;

        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
            && app.handle_key_event(key).await?
        {
            return Ok(());
        }
    }
}

const VERSION: &str = env!("CARGO_PKG_VERSION");
const SILVA_WORKFLOW_HOME: &str = "SILVA_WORKFLOW_HOME";

use std::{io, process, time::{Duration, Instant}};

use crossterm::{event::{DisableMouseCapture, EnableMouseCapture}, execute, terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen}};
use ratatui::prelude::*;

use crate::{envs, ui};
use crate::data_model;

pub async fn run() -> anyhow::Result<()> {
    envs::setup();

    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture, )?;
    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let tick_rate = Duration::from_millis(250);
    let mut last_tick = Instant::now();

    let mut states = ui::States::default();
    let mut store = data_model::Store::default();
    if !store.ac_mgr.get_accounts().is_empty() {
        states.account.list.list.select(Some(0));
    }

    loop {
        terminal.draw(|f| ui::render(f, &mut states, &mut store))?;

        match ui::input(tick_rate, &mut last_tick, &mut states, &mut store).await? {
            ui::Signal::Quit => {
                process::Command::new("reset").status()
                    .unwrap_or_else(|e| panic!("failed to reset terminal with error: {e:?}"));
                break;
            }
            ui::Signal::None => {}
        }

        for (job_id, jh) in states.handlers.iter() {
            if jh.is_finished() {
                let job_local = store.jl_mgr.jobs.get_mut(job_id).unwrap();
                job_local.set_complete();
            }
        }

        states.handlers.retain(|_, jh| !jh.is_finished());
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;

    terminal.show_cursor()?;

    Ok(())
}

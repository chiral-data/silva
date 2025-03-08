use std::{io, time::{Duration, Instant}};
use std::env;

use crossterm::{event::{DisableMouseCapture, EnableMouseCapture}, execute, terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen}};
use ratatui::prelude::*;

use crate::{constants, ui, utils};
use crate::data_model;

async fn setup() {
    let data_dir = utils::dirs::data_dir(); 
    if !data_dir.exists() {
        std::fs::create_dir_all(&data_dir).unwrap();
    }

    // download examples and extract
    // let version = env!("CARGO_PKG_VERSION");
    let tag = "v022";
    let filename = format!("{tag}.tar.gz");
    let url = format!("https://github.com/chiral-data/application-examples/archive/refs/tags/{filename}");
    let filepath = data_dir.join(filename);
    utils::file::download_async(&url, &filepath).await.unwrap();
    utils::file::unzip_tar_gz(&filepath, data_dir.join(tag).as_path()).unwrap();

    if env::var_os(constants::SILVA_PROJECTS_HOME).is_none() {
        panic!("Environment variable SILVA_PROJECTS_HOME is not set and silva cannot start")
    }
}


pub async fn run() -> anyhow::Result<()> {
    setup().await;

    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture, )?;
    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let tick_rate = Duration::from_millis(250);
    let mut last_tick = Instant::now();

    let mut states = ui::states::States::default();
    let mut store = data_model::Store::default();
    states.initialize(&store);
    match store.registry_mgr.initialze(&store.account_mgr, &store.setting_mgr).await {
        Ok(_) => (),
        Err(e) => states.update_info(format!("initialze registry error {e}"), ui::layout::info::MessageLevel::Error)

    }

    loop {
        terminal.draw(|f| ui::home::render(f, &mut states, &mut store))?;

        match ui::home::handle_key(tick_rate, &mut last_tick, &mut states, &mut store).await? {
            ui::home::Signal::Quit => {
                #[cfg(not(windows))]
                {
                    let reset_program = "reset";
                    std::process::Command::new(reset_program)
                        .status()
                        .unwrap_or_else(|e| panic!("failed to reset terminal by {reset_program} with error: {e:?}"));
                }
                // how to reset under windows?
                break;
            }
            ui::home::Signal::None => {}
        }

        // for (job_id, jh) in states.handlers.iter() {
        //     if jh.is_finished() {
        //         let job_local = store.jl_mgr.jobs.get_mut(job_id).unwrap();
        //         job_local.set_complete();
        //     }
        // }

        // states.handlers.retain(|_, jh| !jh.is_finished());
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;

    terminal.show_cursor()?;

    Ok(())
}

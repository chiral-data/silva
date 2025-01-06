use std::{io, path::PathBuf, time::{Duration, Instant}};
use std::env;

use crossterm::{event::{DisableMouseCapture, EnableMouseCapture}, execute, terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen}};
use ratatui::prelude::*;

use crate::{envs, ui, utils};
use crate::data_model;

fn setup() {
    // let xdg_dirs = xdg::BaseDirectories::with_prefix(constants::APP_NAME).unwrap();
    // let data_dir = xdg_dirs.get_data_home();
    let data_dir = utils::file::get_data_dir();
    if !data_dir.exists() {
        std::fs::create_dir_all(data_dir).unwrap();
    }

    // if project home directory is not set, use the directories under "examples"
    if env::var_os(envs::SILVA_PROJECTS_HOME).is_none() {
        let project_homes: String = utils::file::get_child_dirs(PathBuf::from(".").join("examples"))
            .map(|child_dir| child_dir.canonicalize().unwrap().to_str().unwrap().to_string())
            .collect::<Vec<String>>()
            .join(";");
         env::set_var(envs::SILVA_PROJECTS_HOME, project_homes);
    }
}


pub async fn run() -> anyhow::Result<()> {
    setup();

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
                #[cfg(not(target_os = "windows"))]
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

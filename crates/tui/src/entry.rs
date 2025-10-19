use std::{io, time::{Duration, Instant}};

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
    let filename = format!("v{}.tar.gz", constants::TAG);
    let filepath = data_dir.join(&filename);
    if !filepath.exists() {
        println!("download tutorial examples ...");
        let url = format!("https://github.com/chiral-data/application-examples/archive/refs/tags/{filename}");
        utils::file::download_async(&url, &filepath).await.unwrap();
        utils::file::extract_tar_gz(&filepath, data_dir.join(format!("v{}", constants::TAG)).as_path()).unwrap();
        println!("download tutorial examples ... [DONE]");
    }

    if let Ok(projects_dir) = utils::dirs::get_projects_home() {
        if !projects_dir.exists() {
            std::fs::create_dir_all(projects_dir).unwrap();
        }
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

        // Process the job queue periodically to start queued jobs
        let started_jobs = {
            let mut job_mgr = store.job_mgr.lock().unwrap();
            let started_jobs = job_mgr.process_queue();
            
            // Log the started jobs
            for job_id in &started_jobs {
                job_mgr.add_log(*job_id, format!("Job {} started from queue", job_id));
            }
            
            started_jobs
        };
        
        // Execute newly started jobs (outside the lock to avoid deadlock)
        for job_id in started_jobs {
            if let Err(e) = execute_queued_job(job_id, &mut store) {
                let mut job_mgr = store.job_mgr.lock().unwrap();
                job_mgr.add_log(job_id, format!("Failed to execute job {}: {}", job_id, e));
                let _ = job_mgr.update_job_status(job_id, data_model::job::JobStatus::Failed);
            }
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

fn execute_queued_job(job_id: usize, store: &mut data_model::Store) -> anyhow::Result<()> {
    // Get job information
    let (project_path, config_index) = {
        let job_mgr = store.job_mgr.lock().unwrap();
        let job = job_mgr.jobs.get(&job_id)
            .ok_or(anyhow::Error::msg("Job not found"))?;
        
        if !job.is_running() {
            return Ok(()); // Job is not running, nothing to execute
        }
        
        let project_path = job.project_path.clone()
            .ok_or(anyhow::Error::msg("Job has no project path"))?;
        let config_index = job.config_index
            .ok_or(anyhow::Error::msg("Job has no configuration index"))?;
            
        (project_path, config_index)
    };
    
    // Load project from path
    let proj_path = std::path::Path::new(&project_path);
    
    // Get job settings directly from the project directory
    let settings_vec = data_model::job::Job::get_settings_vec(proj_path)
        .map_err(|e| anyhow::Error::msg(format!("Failed to load job settings: {}", e)))?;
    let job_settings = settings_vec.get(config_index)
        .ok_or(anyhow::Error::msg("Invalid configuration index"))?
        .clone();
    
    // Create project with the loaded settings
    let proj = data_model::project::Project::new(proj_path.to_path_buf(), settings_vec);
    
    // Get current registry and pod selection
    let registry_sel = store.registry_mgr.selected(&store.setting_mgr)
        .ok_or(anyhow::Error::msg("no registry selected"))?
        .to_owned();
    
    let pod_sel = store.pod_mgr.selected()
        .ok_or(anyhow::Error::msg("no pod selected"))?;
    
    // Execute job based on pod type
    use data_model::pod::Settings;
    match &pod_sel.settings {
        Settings::Local => {
            let job_mgr = store.job_mgr.clone();
            let job_id_to_cancel = store.cancel_job_id.clone();
            let proj_dir = proj.get_dir().to_path_buf();
            
            tokio::spawn(async move {
                if let Some(local_settings) = job_settings.infra_local {
                    match data_model::provider::local::run_single_job(job_mgr.clone(), job_id_to_cancel, proj_dir, local_settings, job_id).await {
                        Ok(()) => {
                            let mut job_mgr = job_mgr.lock().unwrap();
                            if let Err(e) = job_mgr.update_job_status(job_id, data_model::job::JobStatus::Completed) {
                                job_mgr.add_log(job_id, format!("Failed to update job status: {}", e));
                            }
                        }
                        Err(e) => {
                            let mut job_mgr = job_mgr.lock().unwrap();
                            job_mgr.add_log(job_id, format!("Job execution error: {}", e));
                            if let Err(e) = job_mgr.update_job_status(job_id, data_model::job::JobStatus::Failed) {
                                job_mgr.add_log(job_id, format!("Failed to update job status: {}", e));
                            }
                        }
                    }
                } else {
                    let mut job_mgr = job_mgr.lock().unwrap();
                    job_mgr.add_log(job_id, "No local infrastructure configuration found".to_string());
                    if let Err(e) = job_mgr.update_job_status(job_id, data_model::job::JobStatus::Failed) {
                        job_mgr.add_log(job_id, format!("Failed to update job status: {}", e));
                    }
                }
            });
        },
        Settings::SakuraInternetService(_) => {
            // For cloud execution, we need to implement similar logic
            // This would require more complex handling of cloud parameters
            let mut job_mgr = store.job_mgr.lock().unwrap();
            job_mgr.add_log(job_id, "Cloud execution from queue not yet implemented".to_string());
            if let Err(e) = job_mgr.update_job_status(job_id, data_model::job::JobStatus::Failed) {
                job_mgr.add_log(job_id, format!("Failed to update job status: {}", e));
            }
        },
        Settings::SakuraInternetServer => {
            let mut job_mgr = store.job_mgr.lock().unwrap();
            job_mgr.add_log(job_id, "Server execution not supported".to_string());
            if let Err(e) = job_mgr.update_job_status(job_id, data_model::job::JobStatus::Failed) {
                job_mgr.add_log(job_id, format!("Failed to update job status: {}", e));
            }
        }
    }
    
    Ok(())
}

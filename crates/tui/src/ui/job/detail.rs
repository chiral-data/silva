use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::Mutex;
use ratatui::prelude::*;
use ratatui::widgets::*;
use crossterm::event;

use crate::data_model;
use crate::envs;
use crate::ui;
use crate::utils;

#[derive(Default)]
pub struct States {
}

pub fn render(f: &mut Frame, area: Rect, states: &mut ui::States, store: &data_model::Store) {

}

async fn run_job(proj_dir: PathBuf, image_name: String, base_image: String, job_logs: Arc<Mutex<HashMap<String, Vec<String>>>>) -> anyhow::Result<()> {
    utils::docker::build_image(&proj_dir, &image_name, &base_image, job_logs).await?;
    let (_addr, username, password) = envs::get_sakura_container_registry();
    utils::docker::push_image(&image_name, Some(username), Some(password)).await?;
    
    Ok(())
}

pub fn handle_key(key: &event::KeyEvent, states: &mut ui::States, store: &data_model::Store) {
    use event::KeyCode;

    match key.code {
        KeyCode::Char('r') | KeyCode::Char('R') => {
            // Run the job, for DOK, it will be
            // generate the Dockfile
            // Build the docker image
            // push the docker image
            // submit the task

            if let Some(proj_dir) = &store.proj_selected  {
                let image_name = "example_image:test".to_string();
                let base_image = "nvcr.io/hpc/gromacs:2023.2".to_string();
                let proj_dir = proj_dir.to_owned();
                let job_logs =store.job_mgr.logs.clone();
                tokio::spawn(async move {
                    match run_job(proj_dir, image_name, base_image, job_logs).await {
                        Ok(()) => (),
                        Err(e) => todo!()
                    }
                });
            } else {
                states.info.message = "no project selected".to_string();
            }
        }
        _ => ()
    }
}

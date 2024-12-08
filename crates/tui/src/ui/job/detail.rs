use ratatui::prelude::*;
use ratatui::widgets::*;
use crossterm::event;

use crate::data_model;
use crate::ui;
use crate::utils;

#[derive(Default)]
pub struct States {
}

pub fn render(f: &mut Frame, area: Rect, states: &mut ui::States, store: &data_model::Store) {

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
                tokio::spawn(async move {
                    utils::docker::build_image(&proj_dir, &image_name, &base_image).await.unwrap();
                });
            } else {
                states.info.message = "no project selected".to_string();
            }
        }
        _ => ()
    }
}

use std::path::PathBuf;

use crossterm::event;
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::data_model;
use crate::utils;
use crate::ui;

#[derive(Default)]
pub struct States {
    proj_dir: PathBuf,
    proj_files: Vec<String>,
    list_state_file: ListState,
}

impl States {
    pub fn update(&mut self, store: &data_model::Store) -> anyhow::Result<()> {
        self.proj_dir = utils::project::dir(store)?;
        let job_settings = data_model::job::Job::get_settings(&self.proj_dir)?;
        self.proj_files = job_settings.files.all_files();

        Ok(())
    }
}

pub fn render(f: &mut Frame, area: Rect, states: &mut ui::states::States, _store: &data_model::Store) {
    let current_style = states.get_style(true);
    let states_current = &mut states.project_states.browse;

    ui::components::proj_browser::render(
        f, area, 
        current_style, &states_current.proj_dir, &states_current.proj_files, &mut states_current.list_state_file
    );
}

pub fn handle_key(key: &event::KeyEvent, states: &mut ui::states::States, _store: &mut data_model::Store) {
    let states_current = &mut states.project_states.browse;
    ui::components::proj_browser::handle_key(
        key, 
        &states_current.proj_dir, &states_current.proj_files, &mut states_current.list_state_file
    );
}

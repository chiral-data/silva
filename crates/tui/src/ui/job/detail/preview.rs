use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::data_model;
use crate::ui;
use crate::utils;

pub const HELPER: &[&str] = &[
    "Preview a job", 
    "e.g., generate the docker file and script file for a DOK task for preview", 
];

fn get_file_content(store: &data_model::Store, filename: &str) -> anyhow::Result<()> {
    let proj_dir = super::params::proj_dir(store);

}

pub fn render(f: &mut Frame, area: Rect, states: &mut ui::States, store: &data_model::Store) {
    let current_style = states.get_style(ui::Focus::Main);
    // let states_current = states.job.detail;

    // file list
    let files_string = vec!["Dockerfile", "run.sh"];
    let file_list = List::new(files_string)
        .style(current_style)
        .block(Block::bordered())
        .direction(ListDirection::TopToBottom);

    // file content

    let left_right = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(12),  Constraint::Min(1)]) 
        .split(area);
    let (left, right) = (left_right[0], left_right[1]);
    f.render_widget(file_list, left);
}


pub fn action(states: &mut ui::States, store: &data_model::Store) -> anyhow::Result<()> {
    let proj_dir = super::params::proj_dir(store)?;
    let job_settings = data_model::job::Job::get_settings(&proj_dir)?;
    states.info.message = "Creating job intermediate files ...".to_string();
    utils::docker::prepare_build_files(&proj_dir, &job_settings)?;
    states.info.message = format!("Preview job done for project {}", proj_dir.to_str().unwrap());
    Ok(())
}

use std::path::PathBuf;

use ratatui::prelude::*;
use ratatui::widgets::*;
use crossterm::event;

use crate::data_model;
use crate::ui;
use crate::utils;

pub const HELPER_CLEAR: &[&str] = &[
    "Clear the job project folder", 
    "remove the intermediate files generated by job preview", 
];

#[derive(Default, PartialEq)]
pub enum Tab {
    #[default]
    Preview,
    Clear,
    Run
}

#[derive(Default)]
pub struct States {
    proj_dir: PathBuf,
    job_settings: data_model::job::settings::Settings,
    tab_action: Tab,
    proj_files: Vec<String>,
    list_state_file: ListState,
}

impl States {
    pub fn update(&mut self, store: &data_model::Store) -> anyhow::Result<()> {
        self.proj_dir = params::proj_dir(store)?;
        self.job_settings = data_model::job::Job::get_settings(&self.proj_dir)?;
        let mut build_files_strs = vec!["Dockerfile", "run.sh"]; // file for building docker image
        let mut all_files_strs = self.job_settings.files.all_files();
        all_files_strs.append(&mut build_files_strs);
        self.proj_files = all_files_strs.iter().map(|s| s.to_string()).collect();

        Ok(())
    }
}

pub fn render(f: &mut Frame, area: Rect, states: &mut ui::States, store: &data_model::Store) {
    let current_style = states.get_style(true);
    let states_current = &mut states.job.detail;

    let action_selected = match states_current.tab_action {
        Tab::Preview => 0,
        Tab::Clear => 1,
        Tab::Run => 2,
    };
    let tabs_strings: Vec<String> = ["[P]review", "[C]lear", "[R]un"].into_iter()
        .enumerate()
        .map(|(i, s)| format!("{}{s}", if i == action_selected {
            "[Enter] "
        } else { "" }))
        .collect();
    let actions = Tabs::new(tabs_strings)
        .block(Block::bordered().title(" Actions "))
        .select(action_selected)
        .style(current_style);
    let helper_lines: Vec<Line> = match states_current.tab_action {
        Tab::Preview => preview::HELPER,
        Tab::Clear => HELPER_CLEAR,
        Tab::Run => run::HELPER
    }.iter()
        .map(|&s| Line::from(s))
        .collect();
    let helper = Paragraph::new(helper_lines)
        .style(current_style)
        .block(Block::bordered())
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });


    let top_mid_bottom = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Max(5), Constraint::Min(1)]) 
        .split(area);
    let (top, mid, bottom) = (top_mid_bottom[0], top_mid_bottom[1], top_mid_bottom[2]);

    f.render_widget(actions, top);
    f.render_widget(helper, mid);
    match states_current.tab_action {
        Tab::Preview => preview::render(f, bottom, states, store),
        Tab::Clear => (),
        Tab::Run => run::render(f, bottom, states, store)
    }
}

fn action_clear(states: &mut ui::States, store: &data_model::Store) -> anyhow::Result<()> {
    let proj_dir = params::proj_dir(store)?;
    states.info.message = "Removing job intermediate files ...".to_string();
    utils::docker::clear_build_files(&proj_dir)?;
    states.info.message = format!("File cleaning done for project {}", proj_dir.to_str().unwrap());
    Ok(())
}

pub fn handle_key(key: &event::KeyEvent, states: &mut ui::States, store: &data_model::Store) {
    use event::KeyCode;

    let states_current = &mut states.job.detail;
    match key.code {
        KeyCode::Char('p') | KeyCode::Char('P') => {
            states_current.tab_action = Tab::Preview;
        }
        KeyCode::Char('c') | KeyCode::Char('C') => {
            states_current.tab_action = Tab::Clear;
        }
        KeyCode::Char('r') | KeyCode::Char('R') => {
            states_current.tab_action = Tab::Run;
        }
        KeyCode::Enter => {
            match match states_current.tab_action {
                Tab::Preview => preview::action(states, store),
                Tab::Clear => action_clear(states, store),
                Tab::Run => run::action(states, store)
            } {
                Ok(_) => (),
                Err(e) => states.info.message = format!("job action error: {e}")
            }
        }
        _ => {
            match states_current.tab_action {
                Tab::Preview => preview::handle_key(key, states, store),
                _ => todo!()
            }
        }
    }
}

mod params;
mod preview;
mod run;

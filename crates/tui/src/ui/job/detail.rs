use std::path::PathBuf;

use sacloud_rs::api::dok;
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
    tab_action: Tab,
}


pub fn render(f: &mut Frame, area: Rect, states: &mut ui::States, store: &data_model::Store) {
    let current_style = states.get_style(ui::Focus::Main);
    let states_current = &mut states.job.detail;

    let action_selected = match states_current.tab_action {
        Tab::Preview => 0,
        Tab::Clear => 1,
        Tab::Run => 2,
    };
    let actions = Tabs::new(["[P]review", "[C]lear", "[R]un"])
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

    let job_id = 0;
    let job_mgr = store.job_mgr.lock().unwrap();
    // TODO: use job id 0 for testing first
    let mut logs: Vec<Line> = job_mgr.logs.get(&job_id)
        .map(|v| {
            v.iter()
            .map(|s| s.as_str())
            .map(Line::from)
            .collect()
        })
        .unwrap_or_default();
    if let Some(log_tmp) = job_mgr.logs_tmp.get(&job_id) {
        logs.push(Line::from(log_tmp.as_str()));
    }
    let job_logs = Paragraph::new(logs)
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
    f.render_widget(job_logs, bottom);
}

struct ParametersDok {
    image_name: String, 
    registry: data_model::registry::Registry,
    client: sacloud_rs::Client,
    registry_dok: dok::Registry,
    plan: dok::params::Plan,
}

fn get_job_parameters(store: &data_model::Store, states: &ui::States) -> anyhow::Result<(PathBuf, data_model::job::settings::Settings, ParametersDok)> {
    let proj_dir = store.proj_selected.as_ref()
        .ok_or(anyhow::Error::msg("no project selected"))?;
    let app_sel = store.app_mgr.selected(states)
        .ok_or(anyhow::Error::msg("no application selected"))?;
    let image_name = format!("{}:latest", app_sel.as_str()).to_lowercase();
    let proj_dir = proj_dir.to_owned();
    let client = store.account_mgr.create_client(&store.setting_mgr)
        .ok_or(anyhow::Error::msg("can not create cloud client"))?;
    let registry_sel = store.registry_mgr.selected(&store.setting_mgr)
        .ok_or(anyhow::Error::msg("no registry selected"))?;
    let registry_dok = registry_sel.find_registry_dok(&store.registry_mgr.registries_dok)
        .ok_or(anyhow::Error::msg(format!("can not find registry for Sakura DOK service {:?}", store.registry_mgr.registries_dok)))?;
    let pod_sel = store.pod_mgr.selected()
        .ok_or(anyhow::Error::msg("no pod selected"))?;
    let plan = match &pod_sel.settings {
        data_model::pod::Settings::SakuraInternetServer => { return Err(anyhow::Error::msg("not DOK service")); },
        data_model::pod::Settings::SakuraInternetService(dok_gpu_type) => match dok_gpu_type {
            data_model::provider::sakura_internet::DokGpuType::V100 => dok::params::Plan::V100,
            data_model::provider::sakura_internet::DokGpuType::H100 => dok::params::Plan::H100GB80,
        }
    };
    let job_settings = data_model::job::Job::get_settings(&proj_dir)?;
    let params_dok = ParametersDok { image_name: format!("{}/{image_name}", registry_sel.hostname), registry: registry_sel.to_owned(), client, registry_dok, plan };

    Ok((proj_dir, job_settings, params_dok)) 
}

fn action_clear(states: &mut ui::States, store: &data_model::Store) -> anyhow::Result<()> {
    let (proj_dir, job_settings, _params_dok) = super::get_job_parameters(store, states)?;
    states.info.message = "Creating job intermediate files ...".to_string();
    utils::docker::clear_build_files(&proj_dir, &job_settings)?;
    states.info.message = format!("Preview job done for project {}", proj_dir.to_str().unwrap());
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
                Tab::Clear => todo!(),
                Tab::Run => run::action(states, store)
            } {
                Ok(_) => (),
                Err(e) => states.info.message = format!("job action error: {e}")
            }
        }
        _ => ()
    }
}

mod preview;
mod run;

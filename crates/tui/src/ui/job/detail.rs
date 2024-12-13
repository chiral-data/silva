use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use sacloud_rs::api::dok;
use ratatui::prelude::*;
use ratatui::widgets::*;
use crossterm::event;

use crate::{data_model, run};
use crate::ui;
use crate::utils;

const HELPER_PREVIEW: &[&str] = &[
    "Preview a job", 
    "e.g., generate the docker file and script file for a DOK task for preview", 
];

const HELPER_RUN: &[&str] = &[
    "Launch a job", 
];

#[derive(Default, PartialEq)]
pub enum Tab {
    #[default]
    Preview,
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
        Tab::Run => 1,
    };
    let actions = Tabs::new(["[P]review", "[R]un"])
        .block(Block::bordered().title(" Actions "))
        .select(action_selected)
        .style(current_style);
    let helper_lines: Vec<Line> = match states_current.tab_action {
        Tab::Preview => HELPER_PREVIEW,
        Tab::Run => HELPER_RUN
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

fn prepare_job(store: &data_model::Store, states: &ui::States) -> anyhow::Result<(PathBuf, ParametersDok)> {
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
    let params_dok = ParametersDok { image_name: format!("{}/{image_name}", registry_sel.hostname), registry: registry_sel.to_owned(), client, registry_dok, plan };

    Ok((proj_dir, params_dok)) 
}

async fn launch_job(
    proj_dir: PathBuf, 
    params_dok: ParametersDok,
    job_mgr: Arc<Mutex<data_model::job::Manager>>
) -> anyhow::Result<()> {
    // build & push the docker image
    utils::docker::build_image(&proj_dir, &params_dok.image_name, job_mgr.clone()).await?;
    utils::docker::push_image(params_dok.registry, &params_dok.image_name, job_mgr.clone()).await?;

    // create the task
    let client = params_dok.client.clone();
    let task_created = dok::shortcuts::create_task(client.clone(), &params_dok.image_name, &params_dok.registry_dok.id, params_dok.plan).await?;
    {
        let mut job_mgr = job_mgr.lock().unwrap();
        job_mgr.add_log(0, format!("[sakura internet DOK] task {} created", task_created.id));
    }

    // check task status
    let task = loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
        let task = dok::shortcuts::get_task(client.clone(), &task_created.id).await?;
        let mut job_mgr = job_mgr.lock().unwrap();
        job_mgr.add_log_tmp(0, format!("[sakura internet DOK] task {} status: {}", task.id, task.status));
        if task.status == "done" {
            break task;
        }
    };

    // get artifact url
    let mut count = 0;
    let af_url = loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
        count += 1;
        match dok::shortcuts::get_artifact_download_url(client.clone(), &task).await {
            Ok(af_url) => break af_url,
            Err(_e) => {
                let mut job_mgr = job_mgr.lock().unwrap();
                job_mgr.add_log_tmp(0, 
                    format!("[sakura internet DOK] output files (artifact {}) of task {} not ready {}",
                        task.artifact.as_ref().unwrap().id, task_created.id, ".".repeat(count % 5))
                );
            }
        }
    };

    // download outputs  
    let filepath = proj_dir.join("artifact.tar.gz");
    {
        let mut job_mgr = job_mgr.lock().unwrap();
        job_mgr.add_log(0, format!("[sakura internet DOK] downloading output files of task {}", task_created.id));
    }
    utils::file::download(&af_url.url, &filepath).await?;
    utils::file::unzip_tar_gz(&filepath, &proj_dir)?;
    {
        let mut job_mgr = job_mgr.lock().unwrap();
        job_mgr.add_log(0, format!("[sakura internet DOK] downloaded output files of task {}", task_created.id));
    }
    std::fs::remove_file(&filepath)?;
    
    Ok(())
}

pub fn preview_job() {
    todo!()
}

pub fn run_job(states: &mut ui::States, store: &data_model::Store) {
    match prepare_job(store, states){
        Ok(res) => {
            let (proj_dir, params_dok) = res; 
            let job_mgr = store.job_mgr.clone();
            tokio::spawn(async move {
                match launch_job(proj_dir, params_dok, job_mgr.clone()).await {
                    Ok(()) => (),
                    Err(e) => {
                        let mut job_mgr = job_mgr.lock().unwrap();
                        job_mgr.add_log(0, format!("run job error: {e}"));
                    } 
                }
            });
        }
        Err(e) => states.info.message = format!("get job parameters error: {e}")
    }
}

pub fn handle_key(key: &event::KeyEvent, states: &mut ui::States, store: &data_model::Store) {
    use event::KeyCode;

    let states_current = &mut states.job.detail;
    match key.code {
        KeyCode::Char('p') | KeyCode::Char('P') => {
            states_current.tab_action = Tab::Preview;
        }
        KeyCode::Char('r') | KeyCode::Char('R') => {
            states_current.tab_action = Tab::Run;
        }
        KeyCode::Enter => {
            match states_current.tab_action {
                Tab::Preview => todo!(),
                Tab::Run => run_job(states, store)
            }
        }
        _ => ()
    }
}

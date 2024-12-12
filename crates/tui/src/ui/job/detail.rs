use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use sacloud_rs::api::dok;
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
    let current_style = states.get_style(ui::Focus::Main);

    let actions = vec![
        Line::from("[R]un the new job")
    ];
    let action_list = Paragraph::new(actions)
        .block(Block::bordered().title(" Actions "))
        .style(current_style)
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
    let paragraph = Paragraph::new(logs)
        .block(Block::bordered())
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });

    let top_bottom = Layout::default()
        .direction(Direction::Vertical)
        .constraints([ Constraint::Length(3), Constraint::Min(1)]) 
        .split(area);
    let (top, bottom) = (top_bottom[0], top_bottom[1]);
    f.render_widget(action_list, top);
    f.render_widget(paragraph, bottom);
}

struct ParametersDok {
    image_name: String, 
    base_image: String, 
    registry: data_model::registry::Registry,
    client: sacloud_rs::Client,
    registry_dok: dok::Registry,
    plan: dok::params::Plan,
}

fn prepare_job(store: &data_model::Store) -> anyhow::Result<(PathBuf, ParametersDok)> {
    let proj_dir = store.proj_selected.as_ref()
        .ok_or(anyhow::Error::msg("no project selected"))?;
    let image_name = "gromacs:241211_test";
    let base_image = "nvcr.io/hpc/gromacs:2023.2".to_string();
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
    let params_dok = ParametersDok { image_name: format!("{}/{image_name}", registry_sel.hostname), base_image, registry: registry_sel.to_owned(), client, registry_dok, plan };

    Ok((proj_dir, params_dok)) 
}

async fn run_job(
    proj_dir: PathBuf, 
    params_dok: ParametersDok,
    job_mgr: Arc<Mutex<data_model::job::Manager>>
) -> anyhow::Result<()> {
    let client = params_dok.client.clone();
    utils::docker::build_image(&proj_dir, &params_dok.image_name, &params_dok.base_image, job_mgr.clone()).await?;
    utils::docker::push_image(params_dok.registry, &params_dok.image_name, job_mgr.clone()).await?;

    // create the task
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

pub fn handle_key(key: &event::KeyEvent, states: &mut ui::States, store: &data_model::Store) {
    use event::KeyCode;

    match key.code {
        KeyCode::Char('r') | KeyCode::Char('R') => {
            match prepare_job(store){
                Ok(res) => {
                    let (proj_dir, params_dok) = res; 
                    let job_mgr = store.job_mgr.clone();
                    tokio::spawn(async move {
                        match run_job(proj_dir, params_dok, job_mgr.clone()).await {
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
        _ => ()
    }
}

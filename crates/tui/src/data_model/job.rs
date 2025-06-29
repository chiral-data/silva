//! Jobs run locally to manage cloud infrastructure
//!

use std::{collections::{HashMap, VecDeque}, fs, path::{Path, PathBuf}};

use serde::Deserialize;

use crate::{constants, utils};

#[derive(Debug, Deserialize)]
pub enum JobStatus {
    Created,
    Running,
    Completed,
}

impl std::fmt::Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Created => write!(f, "Created"),
            Self::Running => write!(f, "Running"),
            Self::Completed => write!(f, "Completed"),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub enum Infra {
    None,
    Local,
    // (task id, http uri)
    SakuraInternetDOK(String, Option<String>)
}

#[derive(Debug, Deserialize, Clone)]
pub struct Job {
    pub id: usize,
    pub infra: Infra
}

impl std::fmt::Display for Job {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // write!(f, "{:5} {:8}   {}", self.id, self.status, self.desc)
        write!(f, "{:5} ", self.id)
    }
}

impl Job {
    pub fn new(id: usize) -> Self {
        Self { id, infra: Infra::None }
    }

    pub fn get_settings(proj_dir: &Path) -> anyhow::Result<settings::Settings> {
        let settings_filepath = proj_dir.join("@job.toml");
        let content = std::fs::read_to_string(&settings_filepath)
            .map_err(|e| anyhow::Error::msg(format!("{e}: no settings file {settings_filepath:?}")))?;
        let job_settings = settings::Settings::new(&content)
            .map_err(|e| anyhow::Error::msg(format!("{e}: incorrect settings in {settings_filepath:?}")))?;

        Ok(job_settings)
    }

    pub fn get_settings_vec(proj_dir: &Path) -> anyhow::Result<Vec<settings::Settings>> {
        let single_filepath = proj_dir.join("@job.toml");
        let multiple_filepath = proj_dir.join("@job_1.toml");
        if single_filepath.exists() {
            let content = std::fs::read_to_string(&single_filepath)
                .map_err(|e| anyhow::Error::msg(format!("{e}: no settings file {single_filepath:?}")))?;
            let job_s = settings::Settings::new(&content)
                .map_err(|e| anyhow::Error::msg(format!("{e}: incorrect settings in {single_filepath:?}")))?;
            Ok(vec![job_s])
        } else if multiple_filepath.exists() {
            let mut job_sv = vec![];
            loop {
                let multi_filepath = proj_dir.join(format!("@job_{}.toml", job_sv.len() + 1));
                match std::fs::read_to_string(&multi_filepath) {
                    Ok(content) => {
                        let job_s = settings::Settings::new(&content)
                            .map_err(|e| anyhow::Error::msg(format!("{e}: incorrect settings in {multi_filepath:?}")))?;
                        job_sv.push(job_s)
                    }
                    Err(_e) => break
                }
            }

            Ok(job_sv)
        } else {
            Err(anyhow::Error::msg("no job settings file, single settings with @job.toml and mutilple settings with @job_i.toml"))
        }
    }
}

#[cfg(test)]
fn test_job_settings() {
    let toml_str = r#"
        [files]
        inputs = [
            "a.in",
            "b.in",
            "c.in"
        ]
        outputs = [
            "1.out",
        ]
        scripts = [
            "start.sh",
            "finish.sh"
        ]

        [dok]
        base_image = "a"
        extra_build_commands = ["python load_model.py"]
        http_path = "/dok"
        http_port = 11203
        plan = "v100-32gb"
        "#;

    let temp_dir = std::env::temp_dir();
    let test_dir = temp_dir.join("silva_test_job_settings");
    std::fs::create_dir_all(&test_dir).unwrap();

    // test case 1: no job settings file 
    let proj_dir_1 = test_dir.join("proj_1");
    std::fs::create_dir_all(&proj_dir_1).unwrap();
    let result_err =Job::get_settings_vec(&proj_dir_1);
    assert!(result_err.is_err());

    // test case 2: one file @job.toml under the project folder
    let proj_dir_2 = test_dir.join("proj_2");
    std::fs::create_dir_all(&proj_dir_2).unwrap();
    std::fs::write(proj_dir_2.join("@job.toml"), toml_str).unwrap();
    let job_sv_2 =Job::get_settings_vec(&proj_dir_2).unwrap();
    assert!(job_sv_2.len() == 1);

    // test case 2: 3 files @job_1.toml, @job_2.toml, @job_3.toml under the project folder
    let proj_dir_3 = test_dir.join("proj_3");
    std::fs::create_dir_all(&proj_dir_3).unwrap();
    std::fs::write(proj_dir_3.join("@job_1.toml"), toml_str).unwrap();
    std::fs::write(proj_dir_3.join("@job_2.toml"), toml_str).unwrap();
    std::fs::write(proj_dir_3.join("@job_3.toml"), toml_str).unwrap();
    let job_sv_3 =Job::get_settings_vec(&proj_dir_3).unwrap();
    assert!(job_sv_3.len() == 3);

    // clean
    std::fs::remove_dir_all(&test_dir).unwrap();
}


#[derive(Debug, Deserialize)]
struct DataFile {
    jobs: Option<Vec<Job>>,
}

impl DataFile {
    fn new(content: &str) -> anyhow::Result<Self> {
        let df: Self = toml::from_str(content)?;
        Ok(df)
    }
}

pub struct Manager {
    pub jobs: HashMap<usize, Job>,
    /// job logs: <job id, log contents>
    pub chat_stream: String,
    pub logs: HashMap<usize, VecDeque<String>>,
    pub logs_tmp: HashMap<usize, String>,
    pub local_infra_cancel_job: bool,
}

impl Manager {
    fn data_filepath() -> anyhow::Result<PathBuf> {
        let data_dir = utils::dirs::data_dir();
        let fp = data_dir.join(constants::FILENAME_JOBS);
        Ok(fp)
    }

    pub fn load() -> anyhow::Result<Self> {
        let filepath = Self::data_filepath()?; 
        if !filepath.exists() {
            std::fs::File::create(&filepath)?;
        }

        let content = fs::read_to_string(&filepath)?;
        let df = DataFile::new(&content)?;
        let jobs = match df.jobs {
            Some(jobs) => jobs.into_iter()
                .map(|job| (job.id, job))
                .collect(),
            None => HashMap::new() 
        };

        let chat_stream = String::new();
        let s = Self { 
            jobs, chat_stream, 
            logs: HashMap::new(), logs_tmp: HashMap::new(),
            local_infra_cancel_job: false 
        };
        Ok(s)
    }

    /// add a new log message for job 
    pub fn add_log(&mut self, job_id: usize, log: String) {
        let job_logs = self.logs.entry(job_id).or_default();
        job_logs.push_back(log);
        if job_logs.len() > 20 {
            job_logs.pop_front();
        }
    }

    pub fn add_log_tmp(&mut self, job_id: usize, log: String) {
        let _ = self.logs_tmp.insert(job_id, log);
    }

    pub fn clear_log_tmp(&mut self, job_id: &usize) {
        let _ = self.logs_tmp.remove(job_id);
    }
}

pub mod settings;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ser() {
        let toml_str = r#"
            [[jobs]]
            id = 1 
            infra = "Local"

            [[jobs]]
            id = 2
            infra = "Local"
        "#;
        
        let df = DataFile::new(toml_str).unwrap();
        assert_eq!(df.jobs.unwrap().len(), 2);
    }

    #[test]
    fn run_test_job_settings() {
        test_job_settings();
    }
}


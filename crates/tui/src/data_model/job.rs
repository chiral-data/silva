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
    SakuraInternetDOK(String, Option<String>),
    //(task id, url)
    RustClient(String,String)

}

#[derive(Debug, Deserialize, Clone)]
pub struct Job {
    pub id: usize,
    // status: JobStatus,
    // desc: String,
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

    pub fn get_settings(proj_dir: &Path) ->anyhow::Result<settings::Settings> {
        let settings_filepath = proj_dir.join("@job.toml");
        let job_settings = settings::Settings::new_from_file(&settings_filepath)
            .map_err(|e| anyhow::Error::msg(format!("{e} no settings file {settings_filepath:?}")))?;

        Ok(job_settings)
    }
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
    pub rust_client_cancel_job: bool,
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
            local_infra_cancel_job: false,
            rust_client_cancel_job: false,
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
            status = "Created"
            desc = "some job 1"

            [[jobs]]
            id = 2
            status = "Completed"
            desc = "some job 2"
        "#;
        
        let df = DataFile::new(toml_str).unwrap();
        assert_eq!(df.jobs.unwrap().len(), 2);
    }
}


//! Jobs run locally to manage cloud infrastructure
//!

use std::collections::HashMap;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct JobLocal {
    pub id: usize,
    status: super::common::JobStatus,
    desc: String
}

impl std::fmt::Display for JobLocal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:5} {:8}   {}", self.id, self.status, self.desc)
    }
}

impl JobLocal {
    pub fn new(id: usize, desc: String) -> Self {
        Self { id, desc, status: super::common::JobStatus::Created }
    }

    pub fn set_running(&mut self) {
        self.status = super::common::JobStatus::Running;
    }

    pub fn set_complete(&mut self) {
        self.status = super::common::JobStatus::Completed;
    }
}


#[derive(Debug, Deserialize)]
struct DataFile {
    jobs: Vec<JobLocal>,
}

impl DataFile {
    fn new(content: &str) -> anyhow::Result<Self> {
        let df: Self = toml::from_str(content)?;
        Ok(df)
    }
}

pub struct Manager {
    pub jobs: HashMap<usize, JobLocal>
}

impl Manager {
    pub fn new(content: &str) -> Self {
        let jobs = match DataFile::new(content) {
            Ok(df) => df.jobs.into_iter()
                .map(|job| (job.id, job))
                .collect(),
            Err(_e) => HashMap::new()
        };

        Manager { jobs }
    }
}

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
        assert_eq!(df.jobs.len(), 2);
    }
}


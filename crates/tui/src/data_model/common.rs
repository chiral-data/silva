use serde::Deserialize;

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


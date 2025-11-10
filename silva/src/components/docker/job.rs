use super::logs::LogBuffer;
use chrono::{DateTime, Utc};

/// Status of a Docker job execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobStatus {
    Idle,
    Pending,
    PullingImage,
    BuildingImage,
    CreatingContainer,
    // (container id)
    ContainerRunning(String),
    Running,
    Completed,
    Failed,
}

impl JobStatus {
    pub fn as_str(&self) -> &str {
        match self {
            JobStatus::Idle => "Idle",
            JobStatus::Pending => "Pending",
            JobStatus::PullingImage => "Pulling Image",
            JobStatus::BuildingImage => "Building Image",
            JobStatus::CreatingContainer => "Creating Container",
            JobStatus::ContainerRunning(_) => "Container Created",
            JobStatus::Running => "Running",
            JobStatus::Completed => "Completed",
            JobStatus::Failed => "Failed",
        }
    }

    pub fn is_running(&self) -> bool {
        matches!(
            self,
            JobStatus::PullingImage
                | JobStatus::BuildingImage
                | JobStatus::CreatingContainer
                | JobStatus::ContainerRunning(_)
                | JobStatus::Running
        )
    }

    pub fn is_finished(&self) -> bool {
        matches!(self, JobStatus::Completed | JobStatus::Failed)
    }
}

/// Represents a single job entry with its execution state.
#[derive(Debug, Clone)]
pub struct JobEntry {
    pub name: String,
    pub status: JobStatus,
    pub logs: LogBuffer,
    pub container_id: Option<String>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
}

impl JobEntry {
    pub fn new(name: String) -> Self {
        Self {
            name,
            status: JobStatus::Idle,
            logs: LogBuffer::default(),
            container_id: None,
            start_time: None,
            end_time: None,
            error_message: None,
        }
    }

    // pub fn acquire(&mut self, source: &mut JobEntry) {
    //     self.status = source.status.clone();
    //     self.logs.clear();
    //     self.logs.append(&mut source.logs);
    //     if let Some(id) = source.container_id.take() {
    //         self.container_id = Some(id);
    //     }
    //     if let Some(start_time) = source.start_time.take() {
    //         self.start_time = Some(start_time);
    //     }
    //     if let Some(end_time) = source.end_time.take() {
    //         self.end_time = Some(end_time);
    //     }
    //     if let Some(error_message) = source.error_message.take() {
    //         self.error_message = Some(error_message);
    //     }
    // }

    pub fn start_job(&mut self) {
        self.status = JobStatus::PullingImage;
        self.start_time = Some(Utc::now());
        self.end_time = None;
        self.logs.clear();
        self.container_id = None;
        self.error_message = None;
    }

    pub fn complete_job(&mut self, success: bool) {
        self.status = if success {
            JobStatus::Completed
        } else {
            JobStatus::Failed
        };
        self.end_time = Some(Utc::now());
    }
}

use std::fmt;

/// Error type for Docker operations.
#[derive(Debug)]
pub enum DockerError {
    BollardError(bollard::errors::Error),
    ImageBuildFailed(String),
    ContainerCreateFailed(String),
    ContainerStartFailed(String),
    ScriptExecutionFailed { script: String, exit_code: i64 },
    LogStreamError(String),
    NoContainerId,
    IoError(std::io::Error),
    ChannelSendMessageError(String),
}

impl fmt::Display for DockerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DockerError::BollardError(err) => write!(f, "Docker API error: {err}"),
            DockerError::ImageBuildFailed(msg) => write!(f, "Image build failed: {msg}"),
            DockerError::ContainerCreateFailed(msg) => {
                write!(f, "Container creation failed: {msg}")
            }
            DockerError::ContainerStartFailed(msg) => write!(f, "Container start failed: {msg}"),
            DockerError::ScriptExecutionFailed { script, exit_code } => {
                write!(f, "Script '{script}' failed with exit code {exit_code}")
            }
            DockerError::LogStreamError(msg) => write!(f, "Log streaming error: {msg}"),
            DockerError::NoContainerId => write!(f, "No container ID available"),
            DockerError::IoError(err) => write!(f, "IO error: {err}"),
            DockerError::ChannelSendMessageError(err) => {
                write!(f, "MPSC channel send message error: {err}")
            }
        }
    }
}

impl std::error::Error for DockerError {}

impl From<bollard::errors::Error> for DockerError {
    fn from(err: bollard::errors::Error) -> Self {
        DockerError::BollardError(err)
    }
}

impl From<std::io::Error> for DockerError {
    fn from(err: std::io::Error) -> Self {
        DockerError::IoError(err)
    }
}

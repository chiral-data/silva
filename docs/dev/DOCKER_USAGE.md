# Docker Integration Usage Guide

This project now includes comprehensive Docker support via the Bollard library, allowing you to build images, run containers, and execute jobs with sequential script execution.

## Features

### Core Functionality
- **Image Building**: Build Docker images from Dockerfiles
- **Image Pulling**: Pull Docker images from registries
- **Container Management**: Create, start, and manage containers
- **Sequential Script Execution**: Run pre_run.sh → run.sh → post_run.sh in order
- **Log Streaming**: Real-time log capture with stdout/stderr separation
- **UI Integration**: Visual log viewer with scrolling and status display

### Job Configuration

Jobs are configured via `@job.toml` file:

```toml
[container]
# Option 1: Use a Docker image from a registry
docker_image = "ubuntu:22.04"

# Option 2: Build from a Dockerfile (uncomment to use)
# dockerfile = "./Dockerfile"

[scripts]
# All fields are optional with defaults shown
pre = "pre_run.sh"    # default: "pre_run.sh"
run = "run.sh"        # default: "run.sh"
post = "post_run.sh"  # default: "post_run.sh"
```

## Programmatic Usage

### Basic Example

```rust
use research_silva::docker::DockerExecutor;
use research_silva::job_config::config::JobConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let config = JobConfig::load_from_file("@job.toml")?;

    // Create executor
    let executor = DockerExecutor::new()?;

    // Run job
    let result = executor.run_job(&config).await?;

    // Check result
    if result.success {
        println!("Job completed successfully!");
    } else {
        println!("Job failed");
    }

    // Print logs
    println!("{}", result.logs.to_string());

    Ok(())
}
```

### UI Keyboard Shortcuts

When using the terminal UI:

- **`d`** - Toggle Docker logs popup
- **`↑`** - Scroll logs up
- **`↓`** - Scroll logs down
- **`b`** - Jump to bottom of logs
- **`q`** - Quit application

## Architecture

### Module Structure

```
src/docker/
├── mod.rs          # Module exports
├── error.rs        # Docker-specific errors
├── executor.rs     # Core Docker operations
├── logs.rs         # Log buffer management
└── state.rs        # Job state tracking
```

### Docker Executor

The `DockerExecutor` handles:
- Docker client initialization
- Image building from Dockerfile
- Image pulling from registries
- Container lifecycle management
- Script execution with output capture

### Log Management

Logs are stored in a circular buffer (default 10,000 lines) with:
- Timestamps for each line
- Source identification (stdout/stderr)
- Efficient rotation when buffer fills

### State Management

Job execution tracks:
- Current status (Idle, Building, Running scripts, etc.)
- Start/end timestamps
- Container ID
- Error messages
- Log buffer

## Script Execution

Scripts are executed sequentially inside the container:

1. **Pre-run script** (`pre_run.sh`) - Setup/preparation
2. **Main script** (`run.sh`) - Primary computation
3. **Post-run script** (`post_run.sh`) - Cleanup/finalization

Execution stops immediately if any script fails (returns non-zero exit code).

## Error Handling

The system provides detailed error types:
- `BollardError` - Docker API errors
- `ImageBuildFailed` - Image build failures
- `ContainerCreateFailed` - Container creation errors
- `ScriptExecutionFailed` - Script execution failures with exit codes
- `LogStreamError` - Log streaming issues

## Testing

Run tests with:
```bash
cargo test docker
```

Current test coverage:
- Log buffer operations (push, rotate, tail)
- Job state transitions
- Status tracking
- Error handling

## Requirements

- Docker daemon running locally
- Bollard 0.17+
- Tokio async runtime
- Scripts must be available in container (mount volume or include in image)

## Future Enhancements

Potential improvements:
- Volume mounting for script injection
- Environment variable passing
- Network configuration
- Resource limits (CPU/memory)
- Multiple container orchestration
- Background job execution
- Job queue management

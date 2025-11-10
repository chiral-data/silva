# job_config

Configuration parser for Silva workflow jobs with TOML support.

## Overview

`job_config` provides a simple and type-safe way to parse job configuration files for workflow systems. It supports Docker containers, custom scripts, GPU usage, job dependencies, and file input/output specifications.

## Features

- **Container Configuration**: Support for both Docker images and Dockerfiles
- **Custom Scripts**: Configure pre-run, run, and post-run scripts
- **GPU Support**: Optional GPU usage flag
- **Job Dependencies**: Specify dependencies on other jobs with `depends_on`
- **File I/O**: Define input and output file patterns with glob support
- **TOML Format**: Easy-to-read and write configuration format

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
job_config = "0.3.3"
```

## Usage

### Basic Example

```rust
use job_config::config::JobConfig;

// Load configuration from a TOML file
let config = JobConfig::load_from_file("@job.toml")?;

// Access configuration fields
match &config.container {
    job_config::config::Container::DockerImage(image) => {
        println!("Using Docker image: {}", image);
    }
    job_config::config::Container::DockerFile(path) => {
        println!("Using Dockerfile: {}", path);
    }
}

println!("Run script: {}", config.scripts.run);
```

### TOML Configuration Format

#### Minimal Configuration

```toml
[container]
docker_image = "ubuntu:22.04"
```

#### Full Configuration with Dependencies

```toml
[container]
docker_image = "python:3.11"

[scripts]
pre = "setup.sh"
run = "train.sh"
post = "cleanup.sh"

use_gpu = true

# Job dependencies
depends_on = ["prepare_data", "download_model"]

# Input files from dependency outputs (supports glob patterns)
inputs = ["*.csv", "models/*.pt"]

# Output files to collect (supports glob patterns)
outputs = ["results/*.json", "trained_model.pt"]
```

## Configuration Reference

### Container Section (Required)

Specify either a Docker image or Dockerfile:

```toml
[container]
# Option 1: Docker image
docker_image = "ubuntu:22.04"

# Option 2: Dockerfile path (mutually exclusive with docker_image)
dockerfile = "./Dockerfile"
```

### Scripts Section (Optional)

Customize script names (defaults shown):

```toml
[scripts]
pre = "pre_run.sh"   # Default: "pre_run.sh"
run = "run.sh"       # Default: "run.sh"
post = "post_run.sh" # Default: "post_run.sh"
```

### GPU Support (Optional)

```toml
use_gpu = true  # Default: false
```

### Job Dependencies (Optional)

```toml
depends_on = ["job1", "job2"]
```

### Input Files (Optional)

Specify which files to copy from dependency job outputs:

```toml
# Copy specific patterns
inputs = ["*.csv", "data/*.json"]

# If omitted or empty, all output files from dependencies are copied
inputs = []
```

### Output Files (Optional)

Specify which files to collect after job completion:

```toml
outputs = ["results/*.json", "*.csv", "models/"]
```

## API Documentation

### `JobConfig`

Main configuration structure:

```rust
pub struct JobConfig {
    pub container: Container,
    pub scripts: Scripts,
    pub use_gpu: bool,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub depends_on: Vec<String>,
}
```

### Methods

- `JobConfig::load_from_file(path)` - Load configuration from a TOML file
- Returns `Result<JobConfig, ConfigError>`

## Error Handling

```rust
use job_config::config::{JobConfig, ConfigError};

match JobConfig::load_from_file("@job.toml") {
    Ok(config) => {
        // Use config
    }
    Err(ConfigError::FileNotFound(path)) => {
        eprintln!("Config file not found: {}", path);
    }
    Err(ConfigError::ParseError(msg)) => {
        eprintln!("Failed to parse config: {}", msg);
    }
    Err(e) => {
        eprintln!("Error: {}", e);
    }
}
```

## License

MIT License - see LICENSE file for details.

## Repository

https://github.com/chiral-data/silva

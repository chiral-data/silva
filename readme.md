# Silva TUI - Automate Workflows

A terminal interface for managing and running workflows.

## Documentation

- [Getting Started](doc/get_started.md) - Requirements, installation, FAQ
- [Key Bindings and Navigation](doc/navigation.md)

## Running Workflows

### Usage Overview

The Silva workflow execution system allows you to define and run multi-step workflows using Docker containers. Each workflow consists of multiple jobs that execute sequentially, with each job running in its own Docker container.

### Directory Structure

#### Workflow Home Directory

The workflow home directory is configurable via the `SILVA_WORKFLOW_HOME` environment variable. If not set, it defaults to `./home`.

```bash
export SILVA_WORKFLOW_HOME=/path/to/workflows
```

A collection of workflows can be found in [this repository](https://github.com/chiral-data/collab-workflows).

#### Workflow and Job Structure

```
$SILVA_WORKFLOW_HOME/
├── workflow_1/
│   ├── job_1/
│   │   ├── @job.toml
│   │   ├── pre_run.sh
│   │   ├── run.sh
│   │   └── post_run.sh
│   ├── job_2/
│   │   ├── @job.toml
│   │   ├── Dockerfile
│   │   └── run.sh
│   └── job_3/
│       ├── @job.toml
│       └── run.sh
├── workflow_2/
│   └── job_1/
│       ├── @job.toml
│       └── run.sh
```

### Job Configuration

Each job requires a `@job.toml` configuration file that defines:

#### Container Configuration

You must specify **either** a Docker image URL **or** a Dockerfile (but not both):

#### Using a Docker Image

```toml
[container]
docker_image = "ubuntu:22.04"

[scripts]
run = "run.sh"
```

#### Using a Dockerfile

```toml
[container]
docker_file = "Dockerfile"

[scripts]
pre = "setup.sh"
run = "main.sh"
post = "cleanup.sh"
```

#### Script Configuration

Scripts are optional and have default values:

- `pre`: Pre-execution script (default: `pre_run.sh`), optional
- `run`: Main execution script (default: `run.sh`)
- `post`: Post-execution script (default: `post_run.sh`), optional

**Note 1**: The job folder is mounted as `/workspace` inside the container, and scripts are executed from this directory.

**Note 2**: if pre-execution script and post-execution are not specified, they will be ignored.
help me to improve the expression

#### Complete Example

```toml
[container]
docker_image = "python:3.11-slim"

[scripts]
pre = "install_deps.sh"
run = "process_data.sh"
post = "generate_report.sh"
```

#### Job Dependencies and Data Flow (v0.3.3+)

Jobs can now specify dependencies on other jobs and automatically handle input/output file transfers:

```toml
# Example: A job that depends on preprocessing and uses its outputs
depends_on = ["01_preprocessing", "02_feature_extraction"]
inputs = ["*.csv", "features/*.json"]
outputs = ["model.pkl", "metrics/*.txt"]

[container]
docker_image = "python:3.11-slim"

[scripts]
run = "train_model.sh"
```

**Dependency Fields:**

- `depends_on`: List of job names that must complete before this job runs
  - Jobs execute in dependency order (topological sort)
  - Circular dependencies are detected and reported as errors
  - If a dependency job fails, dependent jobs won't execute

- `inputs`: Glob patterns for files to copy from dependency outputs
  - Files are copied from each dependency's `outputs/` folder
  - If empty or omitted, **all** output files from dependencies are copied
  - Supports wildcards: `*.csv`, `data_*.json`, `results/**/*.txt`
  - Conflicts (same filename from multiple dependencies) use first match with warning

- `outputs`: Glob patterns for files to collect after job execution
  - Matching files are copied to an `outputs/` folder in the job directory
  - Supports wildcards and directory patterns
  - Files become available to jobs that depend on this one
  - If empty, no output collection occurs

**Example Multi-Job Workflow with Dependencies:**

```
ml_pipeline/
├── 01_data_prep/
│   ├── @job.toml         # No dependencies
│   └── prepare.sh        # Outputs: train.csv, test.csv
├── 02_feature_eng/
│   ├── @job.toml         # depends_on: ["01_data_prep"]
│   └── features.sh       # Inputs: *.csv, Outputs: features.json
└── 03_train_model/
    ├── @job.toml         # depends_on: ["02_feature_eng"]
    └── train.sh          # Inputs: features.json, Outputs: model.pkl
```

**01_data_prep/@job.toml:**

```toml
outputs = ["train.csv", "test.csv"]

[container]
docker_image = "python:3.11-slim"

[scripts]
run = "prepare.sh"
```

**02_feature_eng/@job.toml:**

```toml
depends_on = ["01_data_prep"]
inputs = ["*.csv"]
outputs = ["features.json"]

[container]
docker_image = "python:3.11-slim"

[scripts]
run = "features.sh"
```

**03_train_model/@job.toml:**

```toml
depends_on = ["02_feature_eng"]
inputs = ["features.json"]
outputs = ["model.pkl", "metrics.txt"]

[container]
docker_image = "python:3.11-slim"

[scripts]
run = "train.sh"
```

**How It Works:**

1. Jobs execute in dependency order (not alphabetical when dependencies exist)
2. Before a job runs, input files from dependencies are copied to the job directory
3. After successful execution, output files are collected to the `outputs/` folder
4. The workflow displays execution order at startup: `01_data_prep → 02_feature_eng → 03_train_model`

### Creating Workflows

#### 1. Create Workflow Directory

```bash
mkdir -p $SILVA_WORKFLOW_HOME/my_workflow
```

#### 2. Create Job Directories

Job directories should be named in a way that ensures correct execution order (jobs are executed alphabetically by name):

```bash
mkdir -p $SILVA_HOME_DIR/my_workflow/01_preprocessing
mkdir -p $SILVA_HOME_DIR/my_workflow/02_analysis
mkdir -p $SILVA_HOME_DIR/my_workflow/03_reporting
```

#### 3. Create Job Configurations

For each job, create a `@job.toml` file:

```bash
cat > $SILVA_HOME_DIR/my_workflow/01_preprocessing/@job.toml << 'EOF'
[container]
docker_image = "python:3.11-slim"

[scripts]
run = "preprocess.sh"
EOF
```

#### 4. Create Scripts

Create the required scripts (must be executable):

```bash
cat > $SILVA_HOME_DIR/my_workflow/01_preprocessing/preprocess.sh << 'EOF'
#!/bin/bash
set -e

echo "Starting preprocessing..."
python3 -c "print('Preprocessing complete!')"
EOF

chmod +x $SILVA_HOME_DIR/my_workflow/01_preprocessing/preprocess.sh
```

### Workflow Execution Behavior

#### Sequential Execution

- Jobs execute in **dependency order** (topological sort) when dependencies are specified
- For workflows without dependencies, jobs execute in **alphabetical order** by folder name
- Each job runs to completion before the next job starts
- Job folder is mounted as `/workspace` in the container
- Scripts execute with `/workspace` as the working directory
- Input files from dependencies are copied to the job directory before execution
- Output files are collected to the `outputs/` folder after successful execution

#### Script Execution Order

For each job:

1. **Pre-run script** (if specified)
2. **Main run script** (required)
3. **Post-run script** (if specified)

If any script returns a non-zero exit code, the job fails and the workflow stops.

#### Failure Handling

- If a job fails, the workflow stops immediately
- Remaining jobs are not executed
- The failed job name is recorded in the execution result
- Logs up to the point of failure are retained

### Example Workflows

#### Example 1: Data Processing Pipeline

```
data_pipeline/
├── 01_extract/
│   ├── @job.toml
│   └── extract.sh
├── 02_transform/
│   ├── @job.toml
│   ├── Dockerfile
│   └── transform.py
└── 03_load/
    ├── @job.toml
    └── load.sh
```

**01_extract/@job.toml**:

```toml
[container]
docker_image = "alpine:latest"

[scripts]
run = "extract.sh"
```

**02_transform/@job.toml**:

```toml
[container]
dockerfile = "Dockerfile"

[scripts]
run = "transform.py"
```

**02_transform/Dockerfile**:

```dockerfile
FROM python:3.11-slim
RUN pip install pandas numpy
```

#### Example 2: Testing Pipeline

```
test_suite/
├── job_1_unit_tests/
│   ├── @job.toml
│   └── run_tests.sh
├── job_2_integration_tests/
│   ├── @job.toml
│   └── run_tests.sh
└── job_3_e2e_tests/
    ├── @job.toml
    └── run_tests.sh
```

All jobs use the same configuration:

```toml
[container]
docker_image = "node:20-alpine"

[scripts]
pre = "npm install"
run = "npm test"
```

### Troubleshooting

#### Workflow Not Appearing in List

- Verify the workflow directory exists in `$SILVA_WORKFLOW_HOME`
- Press `r` to refresh the workflow list
- Check that jobs contain `@job.toml` files

#### Job Configuration Errors

- Verify `@job.toml` syntax is valid
- Ensure exactly one container type is specified (docker_image OR docker_file)
- Check that script files exist and are executable

#### Docker Execution Errors

- Verify Docker daemon is running
- Check that specified Docker images are available
- Review logs in the Docker popup for detailed error messages
- Ensure scripts have correct shebang (`#!/bin/bash`)

#### Permission Issues

- Make sure all scripts are executable: `chmod +x script.sh`
- Verify Docker has permission to access mounted volumes

### Advanced Usage

#### Sharing Data Between Jobs

**Recommended Approach (v0.3.3+):** Use the `depends_on`, `inputs`, and `outputs` configuration:

```toml
# job_1/@job.toml
outputs = ["result.txt", "data.csv"]

[container]
docker_image = "ubuntu:22.04"

[scripts]
run = "process.sh"
```

```toml
# job_2/@job.toml
depends_on = ["job_1"]
inputs = ["*.txt", "*.csv"]  # or omit to copy all outputs

[container]
docker_image = "ubuntu:22.04"

[scripts]
run = "analyze.sh"
```

Files from job_1's outputs are automatically copied to job_2's directory before execution.

**Legacy Approach:** Access other job folders via relative paths:

```bash
##!/bin/bash
## job_1/run.sh - Write output
echo "result data" > /workspace/output.txt

## job_2/run.sh - Read input
cat /workspace/../job_1/output.txt
```

**Note**: The dependency-based approach is preferred as it makes data flow explicit and handles file copying automatically.

#### Using Environment Variables

Pass environment variables through Dockerfile:

```dockerfile
FROM ubuntu:22.04
ENV MY_VAR=value
```

Or set them in your script:

```bash
##!/bin/bash
export MY_VAR=value
./my_program
```

#### Custom Docker Networks

Currently, each job runs in isolation. For jobs that need to communicate, use file-based data exchange through the workflow directory.

### Best Practices

1. **Name Jobs with Prefixes**: Use numeric prefixes (01*, 02*, 03\_) to ensure correct execution order
2. **Use Set -e**: Always start scripts with `set -e` to fail on errors
3. **Log Verbosely**: Add echo statements to track progress
4. **Test Individually**: Test each job independently before running the full workflow
5. **Keep Jobs Small**: Break complex workflows into smaller, focused jobs
6. **Document Dependencies**: Add README files explaining job purposes and dependencies
7. **Use Specific Tags**: Specify exact Docker image tags (e.g., `ubuntu:22.04` not `ubuntu:latest`)

### Configuration Reference

#### Environment Variables

| Variable              | Default  | Description                  |
| --------------------- | -------- | ---------------------------- |
| `SILVA_WORKFLOW_HOME` | `./home` | Workflow home directory path |

#### File Names

| File          | Required | Description                          |
| ------------- | -------- | ------------------------------------ |
| `@job.toml`   | Yes      | Job configuration file               |
| `run.sh`      | Default  | Main execution script (configurable) |
| `pre_run.sh`  | Default  | Pre-execution script (configurable)  |
| `post_run.sh` | Default  | Post-execution script (configurable) |
| `Dockerfile`  | Optional | Custom Docker image definition       |

#### Exit Codes

| Code     | Meaning                  |
| -------- | ------------------------ |
| 0        | Success                  |
| Non-zero | Failure (workflow stops) |

### Support

For issues or questions:

- Check the logs in the Docker popup (press `d`)
- Review test files for examples
- See source code in `src/components/workflow/` and `src/components/docker/`


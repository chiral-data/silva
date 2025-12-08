# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.7]

### Added

- CLI argument support: run `silva <workflow_path>` to execute a workflow directly in headless mode
- Headless workflow execution outputs logs to stdout/stderr instead of TUI
- Container keep-alive command (`tail -f /dev/null`) for reliable container reuse across jobs

### Changed

- Extracted navigation and key bindings documentation to `doc/navigation.md`
- Extracted requirements, installation, and FAQ to `doc/get_started.md`
- Extracted workflow documentation to `doc/workflows.md`
- Moved release guide to `doc/releasing.md`
- **Configuration unification:**
  - Added new `JobMeta` struct in `job_config/src/job.rs` merging `JobConfig` and `NodeMetadata`
  - Moved `WorkflowMetadata` to separate `job_config/src/workflow.rs` module
  - Job definitions now use TOML format (`job.toml`) with `ParamDefinition` using `toml::Value`
  - Updated callers: `load_config()` → `load_meta()` across workflow and docker components
  - Added new `job_config/src/params.rs` module for JSON-based parameter storage
  - `JobParams` and `WorkflowParams` now use `serde_json::Value` (JSON format)
  - Parameter files: `params.json` for job params, `global_params.json` for workflow params
  - Added `toml_to_json()` and `json_to_toml()` conversion utilities
  - Simplified `Container` struct: now has `image` and `use_gpu` fields (removed DockerFile support)
  - Moved `use_gpu` from `JobMeta` into `Container` struct
  - Moved job dependencies from `JobMeta.depends_on` to `WorkflowMeta.dependencies`
  - Job dependencies are now defined at workflow level in `workflow.toml`
  - Renamed `WorkflowMetadata` to `WorkflowMeta` for consistency with `JobMeta`
  - Removed legacy `config` module (`job_config::config`) - use `job_config::job`, `job_config::params`, and `job_config::workflow` instead
  - Merged `params_editor.rs` and `global_params_editor.rs` into a single generic `ParamsEditorState<T>` using trait-based polymorphism
  - Fixed test race conditions using `serial_test` crate for tests that modify shared env vars
  - Fixed outdated test fixtures to use new `Container` struct format (`image` instead of `docker_image`)
  - Extracted `ParamSource` trait to separate `param_source.rs` module for better code organization
  - Extracted `WorkflowFolder` struct to separate `workflow_folder.rs` module
  - Renamed `Job` to `JobFolder` and `job.rs` to `job_folder.rs` for consistency with `WorkflowFolder`

## [0.3.6]

### Added

- Global workflow parameters support for workflow-level configuration
- Workflow metadata schema in `.chiral/workflow.json` (similar to job-level `node.json`)
- Global parameter values stored in `global_params.json` at workflow root
- Global parameter editor UI accessible via 'g' hotkey
- Parameter merging: global parameters combined with job-level parameters
- Environment variable injection for merged parameters with `PARAM_` prefix
- Enhanced logging showing global, job, and total parameter counts during execution

### Changed

- `run_job()` function signature now accepts both workflow and job parameters
- Parameters are merged with job-level parameters taking precedence over global parameters

## [0.3.5]

### Fixed

- Windows: Fixed CRLF line endings in shell scripts causing execution failures in Linux containers
- Windows: Fixed script path resolution for relative paths (./run.sh patterns)
- Windows: Fixed path operations to use forward slashes for container compatibility
- Windows: Fixed Docker detection using `where` command instead of `which`

## [0.3.4]

### Added

- Job parameters support with interactive parameter editor UI
- Parameter types: string, integer, float, boolean, file, directory, enum, and array
- Parameter definitions in `.chiral/node.json` files
- Parameters loaded from `params.json` and injected as environment variables with `PARAM_` prefix
- Parameter editor accessible via 'p' hotkey with real-time validation

### Changed

- License changed from MIT to Mozilla Public License Version 2.0 (MPL-2.0)
- Job configuration file location from `@job.toml` to `.chiral/job.toml` (legacy location still supported)
- Job run hotkey changed from 'r' to 'Enter' for better usability

## [0.3.3]

### Added

- Job dependencies with `depends_on` field and topological sorting
- Input/output file patterns with glob support (`*.csv`, `data/*`)
- Recursive directory copying for input files
- Container reuse by image for improved performance

### Changed

- Extracted `job_config` as standalone publishable crate
- Restructured project as Cargo workspace

## [0.3.2]

- build images (2025-11-02): Dockerfile file path, Avoid rebuilding the image

## [0.3.1]

### Fixed

- Windows: Fixed double keystroke registration by filtering key press/release events
- Windows: Fixed PowerShell install script returning incorrect version string
- Windows: Improved emoji display compatibility
- Fixed CPU usage display format to show one decimal place

## [0.3.0] - 2025

### Added

- Initial release with workflow automation support
- Docker container management
- Terminal UI with multiple tabs (Applications, Workflows, Settings)
- Health check system monitoring
- Real-time log viewing for Docker jobs
- Multi-job workflow support

### Changed

- Updated dependencies and project structure

## [0.2.4]

### Fixed

- Various bug fixes and improvements

## [0.2.3]

### Fixed

- Docker environment variable handling

## [0.2.2]

### Fixed

- Tag naming issues
- Various stability improvements

## [0.2.1]

### Added

- Feature enhancements

## [0.1.0]

### Added

- Initial project setup
- Basic TUI framework
- Core workflow management features

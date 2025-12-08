# Tasks: configuration unification

## Steps

- [x] safely remove job_config/src/config.rs
  1. Removed pub mod config; from job_config/src/lib.rs
  2. Deleted job_config/src/config.rs (893 lines of legacy code)
  3. Updated CHANGELOG.md to document the removal of the legacy config module
  4. Updated job_config/README.md to:
  - Fix outdated example code that referenced Container::DockerImage/Container::DockerFile enum variants
  - Update Features section to reflect current architecture (Docker image with GPU support, not Dockerfiles)
- [x] safely merge `silva/src/components/workflow/params_editor` and `silva/src/components/workflow/global_params_editor.rs`
  1. Created `ParamSource` trait abstracting common interface for parameter sources
  2. Implemented `JobParamSource` (wraps Job + JobMeta) and `WorkflowParamSource` (wraps WorkflowFolder + WorkflowMeta)
  3. Refactored `ParamsEditorState<T: ParamSource>` to be generic over the source type
  4. Updated render functions to be generic, with dynamic title/label based on `is_global()`
  5. Updated callers in `state.rs` and `layout.rs` to use the new generic types
  6. Deleted `global_params_editor.rs` (396 lines removed)
- [x] run tests by `cargo test --workspace` and analyse why tests fail.

  **Analysis Results:** 12 tests failing in `silva` crate (all `job_config` tests pass)

  **Root Cause 1: Test Race Conditions (Environment Variable Conflicts)**
  - Tests in `manager.rs` and `home.rs` use shared env var `SILVA_WORKFLOW_HOME`
  - Rust runs tests in parallel by default
  - Multiple tests modifying the same env var concurrently causes interference
  - Example: `test_scan_workflows_empty_directory` expects 0 workflows but gets 2
  - **Fix:** Use `#[serial]` from `serial_test` crate, or use unique paths per test

  **Root Cause 2: Outdated Config Format in Tests**
  - Tests in `job.rs` use old format: `docker_image = "ubuntu:22.04"`
  - New `Container` struct expects: `image = "ubuntu:22.04"`
  - Causes `load_meta()` to fail with TOML parse error
  - Affected tests: `test_job_load_meta`, `test_job_has_config`, `test_is_job_folder`
  - **Fix:** Update test fixtures to use `[container]\nimage = "ubuntu:22.04"`

  **Failing Tests Summary:**
  - `home.rs`: `test_absolute_path`, `test_not_a_directory_error` (race conditions)
  - `job.rs`: `test_job_has_config`, `test_job_load_meta`, `test_is_job_folder`, `test_scan_jobs_*` (config format + races)
  - `manager.rs`: `test_scan_workflows_*`, `test_refresh_workflows`, `test_create_workflow_sanitizes_name` (race conditions)

  **Fixes Applied:**
  1. Added `serial_test = "3.1"` as dev-dependency
  2. Added `#[serial]` attribute to all tests that modify env vars or shared paths
  3. Fixed outdated config format: `docker_image` -> `image` in test fixtures
  4. All 58 tests now pass

- [x] move the `ParamSource` out of `silva/src/components/workflow/params_editor.rs` and put into a new file `silva/src/components/workflow/param_source.rs`
  1. Created `param_source.rs` with `ParamSource` trait, `JobParamSource`, and `WorkflowParamSource`
  2. Updated `params_editor.rs` to import from `param_source` module
  3. Updated `mod.rs` to export from new module location
- [x] move the `WorkflowFolder` out of `silva/src/components/workflow/manager.rs` and put into a new file `silva/src/components/workflow/workflow_folder.rs`
  1. Created `workflow_folder.rs` with `WorkflowFolder` struct and all its methods
  2. Updated `manager.rs` to import from `workflow_folder` module
  3. Updated `param_source.rs` to import from `workflow_folder` module
  4. Updated `mod.rs` to export from new module location
- [x] rename `Job` in `silva/src/components/workflow/job.rs` to `JobFolder` and rename the file to `silva/src/components/workflow/job_folder.rs`
  1. Renamed `job.rs` to `job_folder.rs`
  2. Renamed `Job` struct to `JobFolder`
  3. Updated all imports and references in: `mod.rs`, `param_source.rs`, `docker/state.rs`, `docker/executor.rs`, `examples/docker_executor.rs`
  4. Renamed test functions: `test_job_new` -> `test_job_folder_new`, etc.
- [x] add argument for "silva".
  - If "silva" launches without any argument, the TUI app will start.
  - If "silva" launches with an argument of a workflow folder, it will run the workflow directly and output the stdout and stderr to the user.
  1. Added `clap = "4.5"` dependency with derive feature to silva/Cargo.toml
  2. Created `silva/src/headless.rs` module with `run_workflow()` function for headless execution
  3. Updated `silva/src/lib.rs` to export the new `headless` module
  4. Updated `silva/src/main.rs` with CLI argument parsing:
     - Without args: starts TUI application
     - With workflow path: runs workflow directly, outputs to stdout/stderr
  5. Added to CHANGELOG.md documenting the new CLI argument support
- [] add progress info when docker is pulling an image
- [] publish the crate `job_config` to crate.io

## Rules for each step

- Some steps will be done manally and they will be marked and highlighted.
- [x] means it has been done.
- After the completion of each step, update "CHANGELOG.md" and the "doc/\*.md" and also "job_config/README.md"

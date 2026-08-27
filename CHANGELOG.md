# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- CI can now fail when tests fail (#106). The silva test step carried `continue-on-error: true`, justified in a comment as *"Silva has some pre-existing test failures"* — there are none. Measured under the exact command CI runs: **112 passing, 0 failing** (104 lib, 2 pre-check integration, 6 Docker integration). The setting was discarding a working signal, including the only tests that run a workflow end to end.
- A real published workflow now runs on every build (#106). A new `workflow-e2e` job checks out `collab-workflows` beside silva and runs `test_workflows.sh`, which executes `workflow-007` headlessly — three dependent nodes, all on the public `ghcr.io/chiral-data/pocketeer` image, pulled once. silva exits non-zero when a job fails, so this is a real gate rather than a report.

  Two deliberate trade-offs, recorded so they are not mistaken for oversights. `01-download` fetches from `files.rcsb.org`, so an outage there can fail the job with no silva change — and because the job is blocking on pull requests, it will block them while it lasts. If that becomes a problem the answer is a retry or a lighter fixture, **not** re-adding `continue-on-error`. And `collab-workflows` is tracked at its default branch rather than pinned, so silva is verified against the workflows users actually run, at the cost of an external commit being able to turn CI red.

  Note what this gate does and does not prove: that silva loads a real workflow, resolves its dependency order, pulls the image, runs three containers and reports success. It does not check the scientific output — `03-visualize` currently collects zero files and still completes, which is the workflow's business rather than silva's.

### Fixed

- A missing Docker daemon no longer reports the integration tests as passing (#106). `require_docker()` called `std::process::exit(0)`, which ended the whole test binary with a **success** status: the harness printed `running 6 tests`, one truncated line, then nothing — no summary, exit code 0, and `cargo test` reported a pass. Five of the six tests in `integration_complete.rs` called it, so one absent daemon silently took four others down with it.

  It is now `docker_available() -> bool`, which each test branches on and returns from itself, so all six are reported — one running, five printing `[SKIP]`.

## [0.5.12]

### Added

- Updates install themselves, without asking (#101)
  - A patch release is fetched in the background, verified, and renamed into place while the current binary keeps running; it becomes the running version on the next start, which says `Now running silva v0.5.12, updated from v0.5.11.` once. Separating install from activation is what makes an unattended update safe — nothing changes underneath a workflow, so the version that starts a run is still the version that finishes it
  - `silva --rollback` restores the binary the last update replaced. It is kept next to the new one (a hard link, so it costs nothing), and restoring it needs no network
  - `SILVA_UPDATE=auto|notify|off` sets the policy; `auto` is the default for every kind of invocation — TUI, `silva run`, `--json`, terminal or pipe. `--no-update` and the existing `SILVA_NO_UPDATE_CHECK`/`NO_UPDATE`/`CI` opt-outs still switch it off entirely, and an explicit `SILVA_UPDATE` overrides them
  - `release.yml` now publishes a `.sha256` beside every release asset, which is what an update verifies against

### Changed

- The confirmation prompt is gone, along with the reasons it existed (#101)
  - Nothing is asked of the user at any point, and no invocation self-modifies mid-run
  - Installing no longer shells out to `curl … install.sh | sh`. The release asset is downloaded directly and checked against its published SHA-256, so the trust anchor is a digest rather than whatever is on the default branch at that moment. A download that does not match is discarded
  - Only within a patch series: a minor or major release is reported rather than installed, since that is the boundary where behaviour is allowed to change
  - Only when silva's own binary is writable, so a packaged or system-wide install is left to whoever owns it
  - The version check is cached for a day, so it costs roughly one request a day rather than one per invocation. The TUI's update badge is read from that cache, so the first frame no longer waits on a network request
  - Under `--json`, an update reports itself as a `note` event, which may arrive after the terminal `workflow` event

## [0.5.11]

### Fixed

- Self-update no longer fails on its own running binary, and no longer prompts inside a batch run (#101)
  - `install.sh` replaced its target with `cp`, which cannot write an executable that is currently running (`ETXTBSY`, "Text file busy") — so an update triggered from a running silva always failed. The new binary is now staged as a dotfile in the destination directory and `mv`'d over the target: the rename is atomic, the running process keeps its unlinked inode, and the next invocation is the new version
  - `install.ps1` had the same defect in Windows form (`Copy-Item -Force` hits a sharing violation on a running image). Windows does allow the running image to be renamed, so the old binary is moved aside first, then the new one is copied into the freed name, with the previous binary restored if that copy fails
  - The startup check was gated on `--json` alone, so a scripted human-output run (`silva run <dir>` with stdin piped) still printed `Update now? [Y/n]:` and read the pipe as an answer — then tried to install a new binary over the one about to run the workflow

### Changed

- What an invocation may do about updates is now decided before the check runs, from the shape of the invocation (#101)
  - A workflow run reports an available version and proceeds on the current one — it never prompts and never installs, so the version that starts a run is the version that finishes it
  - Stdin not being a terminal disables the check outright, rather than leaving a prompt to be answered by whatever the pipe happens to contain
  - New `--no-update` flag and `SILVA_NO_UPDATE_CHECK` / `NO_UPDATE` / `CI` env opt-outs skip the check entirely, so silva makes no outbound request of its own — relevant to workflows whose premise is local execution with an accounted-for network ledger
  - Unchanged for the case the feature was built for: starting the TUI on a terminal still checks, prompts, and installs

## [0.5.10]

### Fixed

- Headless mode (`silva run`): Ctrl-C during a run now stops and removes the containers the run created, instead of leaving them behind (#99)
  - Headless discarded the cancel sender it created, so the cancellation support `DockerExecutor::run_job`/`exec_script` already honour was unreachable — SIGINT just killed the process before `cleanup_containers` could run
  - Ctrl-C is now wired to a real sender via a `tokio::signal::ctrl_c()` handler, so it unwinds through the existing job loop into cleanup, and the run reports itself as cancelled (rather than failed) — in human output and as a terminal `{"event":"workflow","status":"cancelled",...}` line under `--json`
  - A second Ctrl-C hard-exits immediately, in case cleanup itself is stuck on a container that will not stop

## [0.5.9]

### Added

- CLI: `--json` on `silva run` — emit a headless run as newline-delimited JSON events instead of human output (#97)
  - Four event types: `workflow` (started/completed/failed with the output folder), `job` (phase changes, then exactly one terminal state), `log` (one container line, attributed to its job and stream), and `note` (silva's own diagnostics)
  - Jobs an aborted run never reached are reported as `skipped`, so "never ran" is distinguishable from "not part of this workflow"
  - Exactly one terminal event per job: the executor reports `Completed` once per script, which is progress rather than a job finishing, so terminal state is decided by the run moving on
  - Every line on stdout is a JSON object — diagnostics that used to be printed directly, including the `silva <WORKFLOW_PATH>` deprecation warning, become `note` events rather than unparseable lines
  - The startup update check is skipped: it prints human text onto the stream, prompts for input, and reaches the network
  - Flushed per event, so a consumer following a long run sees progress as it happens
  - Human output is unchanged, verified line for line against a run on the previous build

### Known limitations

- Job events carry silva's own failure message in `error` but not a numeric exit code. `DockerError::ScriptExecutionFailed` holds one, but the channel between the executor and the reporting side carries `(index, JobStatus, LogLine)` and has no field for it; adding one touches every send site in the TUI as well. Deferred deliberately rather than inferred from log text.

## [0.5.8]

### Added

- CLI: `silva run <WORKFLOW_PATH>` — explicit subcommand for headless mode, alongside `silva validate` (#95)
  - `silva <WORKFLOW_PATH>` (bare positional) still works but is deprecated; it now prints a one-line warning pointing at `silva run`

## [0.5.7]

### Added

- CLI: `silva validate <WORKFLOW_PATH>` — check a workflow folder without running it (#93)
  - Parses `workflow.toml` and every `job.toml`, resolves the dependency graph and reports the execution order, validates `global_params.json` and each job's `params.json` against their `[params]` definitions, and applies the existing prechecks (install commands, cross-node `../` references, `input_files/`)
  - Reuses `precheck` and the headless topological sort, so validation cannot disagree with what a run enforces
  - Reports every unresolvable dependency name rather than only the first, and excludes a job whose `job.toml` does not parse from later checks so one broken file does not cascade
  - Exit `0` when sound, `1` when not; `--json` emits a structured report with a `kind` taxonomy (`workflow`, `job`, `dependency`, `params`, `script`, `inputs`)
  - Reads the folder in place rather than the temp copy a run uses, so paths in messages are the paths being edited
  - Performs no update check, needs no Docker daemon and no network — usable in CI

## [0.5.6]

### Added

- `infra::dok` (headless mode only): automatic bundle preparation for `RUN_MODE=use_dok` jobs (#91)
  - Detects `RUN_MODE=use_dok` in a job's resolved env vars, tars the job's own directory (script files + its already-merged `inputs/` subdirectory, excluding only `outputs/`), and submits a tiny prep task to Sakura's DOK API whose `command` decodes and deposits the payload as a DOK artifact
  - Injects the resulting presigned `DOK_BUNDLE_URL` into the job's env vars before launching it — no manual bundle construction needed
  - Payload travels via `command`, not `environment` — DOK's `environment` field is capped at 8192 total characters (confirmed live), which even a small bundle's base64 payload exceeds; `command` was confirmed live to accept 200KB+
  - Requires silva's own `SAKURA_ACCESS_TOKEN`/`SAKURA_ACCESS_TOKEN_SECRET` (read from silva's host environment)
  - Verified live end-to-end against the real DOK API: a full 5-node dependency chain, including a 3-way dependency merge, all through the real `silva` binary

## [0.5.5]

### Added

- CLI: `-e/--env KEY=VALUE` flag (repeatable, headless mode only) — set ad-hoc environment variables in every job's container for a single run (#89)
  - Injected unprefixed into the container exec environment, alongside `PARAM_*` and `env_passthrough` values
  - Independent of `env_passthrough`'s allowlist — always applied regardless of `workflow.toml` contents
  - Malformed entries (missing `=`) are rejected before any container runs

## [0.5.4]

### Added

- `job_config`: `env_passthrough` field in `workflow.toml` — forward host environment variables into the container exec environment (#86)
  - Host env vars listed by name (e.g. `NGC_API_KEY`, `HF_TOKEN`) are read via `std::env::var()` and appended to the container's env vars alongside the existing `PARAM_*` values
  - Lets a workflow require API keys or secrets without hardcoding them into `global_params.json`
  - A listed variable not set in the host environment is silently skipped

## [0.5.3]

### Fixed

- Brace expansion in `inputs` patterns now works correctly (#82)
  - The `glob` crate silently returned zero matches for patterns like `*.{fasta,fa,faa}`; replaced with `globset` which supports brace expansion natively
  - Affects both headless and TUI execution paths (`headless.rs` and `state.rs`)

## [0.5.2]

### Added

- `job_config`: `registry = "local"` field in `[container]` — opt-out of default registry resolution (#80)
  - Locally-built Docker images (e.g. `aso-rna:latest`) that do not exist in any remote registry can now be marked with `registry = "local"` to bypass the worker's `default_registry` prefix logic and skip `docker pull`
  - Introduces `ImageSource::LocalImage` variant alongside existing `Registry`, `TarFile`, and `SifFile`

## [0.4.0]

### Changed

- **BREAKING**: Input files are now copied to `inputs/` subfolder instead of job root
  - Dependency outputs are copied to `{job}/inputs/` folder
  - Workflow `input_files/` contents are copied to first job's `inputs/` folder
  - Scripts should read from `./inputs/` and write to `./outputs/`

### Added

- Input files feature: workflows can now include an `input_files/` folder at the root, whose contents are automatically copied to the first job's `inputs/` folder before execution

## [0.3.8]

### Added

- Local Docker image detection: `pull_image` now checks if an image exists locally before attempting to pull from registry
- Test coverage for local Docker image detection feature
- Auto-update feature: checks GitHub releases on startup, prompts user to update if new version available, shows notification in TUI footer if update is deferred

### Changed

- Refactored `run_job` function signature to use tuple grouping for related parameters

### Fixed

- Workflow now correctly reports `Failed` status when a job fails (was incorrectly reporting `Completed`)
- `run_job` now returns error when script execution fails, stopping workflow immediately instead of continuing to next job

## [0.3.7]

### Added

- CLI argument support: run `silva <workflow_path>` to execute a workflow directly in headless mode
- Headless workflow execution outputs logs to stdout/stderr instead of TUI
- Container keep-alive command (`tail -f /dev/null`) for reliable container reuse across jobs
- Headless mode now copies input files from dependency outputs to current job folder
- Headless mode creates temp folder for workflow execution (mirrors TUI behavior)
- Example workflow-007 (Protein Pocket Analysis) with parameterized configuration:
  - Global `pdb_id` parameter in workflow.toml
  - Job-level parameters for pocketeer.find_pockets (r_min, r_max, polar_probe_radius, etc.)
  - Enum parameters for visualization options (pocket_style, render_method, representation, output_format)
- Docker image pull progress display: shows layer ID, download/extract status, and percentage
- `ImageSource` enum in `job_config` to support multiple image sources: Docker registry, local tar files (.tar), and Singularity/Apptainer images (.sif)

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

### Fixed

- Docker `pull_image` now checks if image exists locally before pulling, avoiding unnecessary network requests

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

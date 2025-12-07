# Tasks: configuration unification

## Steps

- [x] safely remove job_config/src/config.rs
  1. Removed pub mod config; from job_config/src/lib.rs
  2. Deleted job_config/src/config.rs (893 lines of legacy code)
  3. Updated CHANGELOG.md to document the removal of the legacy config module
  4. Updated job_config/README.md to:
  - Fix outdated example code that referenced Container::DockerImage/Container::DockerFile enum variants
  - Update Features section to reflect current architecture (Docker image with GPU support, not Dockerfiles)
- [] add more tests for `WorkflowMeta` and `JobMeta`.
- [] merge params_editor and global_params_editor.rs
- [] publish the crate `job_config` to crate.io

## Rules for each step

- Some steps will be done manally and they will be marked and highlighted.
- [x] means it has been done.
- After the completion of each step, update "CHANGELOG.md" and the "doc/\*.md" and also "job_config/README.md"

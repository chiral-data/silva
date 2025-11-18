# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

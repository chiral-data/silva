# Changelog
All notable changes will be documented in this file.

## [TODO]
- [] job management
- [] SSH accessible servers as a computation pod 
- [] integrate quantum-expresso



## [Unreleased] v0.2.5 chiral infra



## [2025-06-12] v0.2.4 local infra
### Added
- [x] local as infra 
    - [x] start a job
    - [x] cancel a job
- [x] add "version" at the setting page
### Changed
- [x] fix root README.md
### Removed
- [x] remove the folders: src/sakura 



## v0.2.3 

### Added
- [x] tui: add "commands" in dok parameter
### Changed
### Removed
- [x] remove the folders: ui/infra, ui/job, ui/project, ui/settings



## v0.2.2 llm with ollama and dok
integrate Ollama; binary release;

### Added
- [x] setup of sacloud API tokens, 
- [x] support application-examples from chiral
- [x] default registry chiral.sakuracr.jp 
- [x] talk to ollama server via [ollama-rs](https://github.com/pepperoni21/ollama-rs)
- [x] chat UI: scrolling, display Markdown
- [x] example "ollama_dok" without docker image building
- [x] a pull-only user of chiral.sakuracr.jp for public use

### Changed
- [x] Dockerfile automatical generating deprecrated
- [x] refactor project dirs
- [x] remove crate home

### Removed



## 0.2.1

### Added
- [x] automatically add dok registries
- [x] @job.toml, @pre.sh, @post.sh used as default job setting files; move pod selection out as a seperate process
- [x] gromacs benchmark example: [A free GROMACS benchmark set](https://www.mpinat.mpg.de/grubmueller/bench) by Max Planck Institute

### Fixed


### Other


## 0.1.0 - First Publishing

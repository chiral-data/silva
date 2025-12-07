// # Example job configuration file
// # This file demonstrates the job configuration format
//
// [container]
// # Option 1: Use a Docker image
// docker_image = "ubuntu:22.04"
//
// # Option 2: Use a Dockerfile (uncomment to use instead)
// # dockerfile = "./Dockerfile"
//
// [scripts]
// # All script fields are optional with default values:
// # - pre: "pre_run.sh" (default)
// # - run: "run.sh" (default)
// # - post: "post_run.sh" (default)
//
// # Custom script names (optional)
// pre = "setup.sh"
// run = "compute.sh"
// post = "cleanup.sh"

pub mod config;
pub mod job;
pub mod workflow;

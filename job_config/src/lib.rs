// # Example job configuration file
// # This file demonstrates the job configuration format
//
// [container]
// image = "ubuntu:22.04"        # registry image
// # image = "./image.tar"       # Docker tar archive
// # image = "./container.sif"   # Singularity/Apptainer image
// # registry = "local"          # locally-built; skip registry resolution
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

pub mod job;
pub mod params;
pub mod workflow;

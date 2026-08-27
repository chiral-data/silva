//! Discovery of local application images shipped inside a workflow folder.
//!
//! A workflow may carry its own `apps/` directory holding one Dockerfile
//! directory per image it needs:
//!
//! ```text
//! workflow-028/
//! ├── apps/
//! │   ├── p2rank_2026_07_10/Dockerfile
//! │   └── pipeline-report_2026_07_10/Dockerfile
//! ├── 01-pockets/
//! └── 02-report/
//! ```
//!
//! Those images exist in no registry, so a job referencing a bare
//! `p2rank:2026_07_10` cannot be satisfied by a pull. This module locates the
//! definitions and works out which of them a given workflow actually needs, so
//! the run can build them first.

use std::fs;
use std::path::{Path, PathBuf};

use crate::components::workflow::JobFolder;

/// A local application image definition found under a workflow's `apps/` folder.
#[derive(Debug, Clone, PartialEq)]
pub struct LocalApp {
    /// Directory name as it appears under `apps/`, e.g. `admet_pipeline_2026_06_02`.
    pub dir_name: String,
    /// Image tag derived from `dir_name`, e.g. `admet_pipeline:2026_06_02`.
    pub image: String,
    /// Path to the Dockerfile that builds this image. Its parent is the build context.
    pub dockerfile: PathBuf,
}

/// Derives an image tag from an `apps/` directory name.
///
/// Splits on the last `_` followed by four digits: everything before it becomes
/// the image name, everything after becomes the tag. A trailing `_v<n>` stays
/// part of the tag.
///
/// | Directory | Derived tag |
/// |---|---|
/// | `p2rank_2026_07_10` | `p2rank:2026_07_10` |
/// | `admet_pipeline_2026_06_02` | `admet_pipeline:2026_06_02` |
/// | `pipeline-report_2026_07_10` | `pipeline-report:2026_07_10` |
///
/// Note this is deliberately *not* the convention in `collab-workflows`'
/// `apps/build.sh`, which cuts on the first underscore and so mis-derives every
/// multi-word app name. The convention here is the one the job configs use.
///
/// Returns `None` when the name has no such boundary, or when either side of it
/// would be empty — such a directory is not a recognised app definition.
pub fn derive_image_tag(dir_name: &str) -> Option<String> {
    let bytes = dir_name.as_bytes();

    let split = dir_name
        .char_indices()
        .filter(|&(i, c)| {
            c == '_'
                && bytes
                    .get(i + 1..i + 5)
                    .is_some_and(|d| d.iter().all(u8::is_ascii_digit))
        })
        .map(|(i, _)| i)
        .next_back()?;

    let (name, tag) = (&dir_name[..split], &dir_name[split + 1..]);
    if name.is_empty() || tag.is_empty() {
        return None;
    }

    Some(format!("{name}:{tag}"))
}

/// Scans `<workflow_path>/apps/` one level deep for directories holding a
/// `Dockerfile`, and derives each one's image tag.
///
/// Returns an empty vec when the workflow ships no `apps/` folder, which is the
/// common case — such workflows are unaffected by any of this.
///
/// Results are sorted by directory name so build order is deterministic.
pub fn discover_apps(workflow_path: &Path) -> Vec<LocalApp> {
    let apps_dir = workflow_path.join("apps");

    let Ok(entries) = fs::read_dir(&apps_dir) else {
        return Vec::new();
    };

    let mut apps: Vec<LocalApp> = entries
        .filter_map(Result::ok)
        .filter(|e| e.path().is_dir())
        .filter_map(|entry| {
            let dockerfile = entry.path().join("Dockerfile");
            if !dockerfile.is_file() {
                return None;
            }
            let dir_name = entry.file_name().to_string_lossy().into_owned();
            let image = derive_image_tag(&dir_name)?;
            Some(LocalApp {
                dir_name,
                image,
                dockerfile,
            })
        })
        .collect();

    apps.sort_by(|a, b| a.dir_name.cmp(&b.dir_name));
    apps
}

/// Collects the `container.image` value of every job that has a readable config.
///
/// Jobs whose config fails to load are skipped rather than reported: the run
/// path loads each config again and surfaces the error there, with the job
/// index attached.
fn referenced_images(jobs: &[JobFolder]) -> Vec<String> {
    jobs.iter()
        .filter_map(|job| job.load_meta().ok())
        .map(|meta| meta.container.image)
        .collect()
}

/// Returns the local apps this workflow actually needs — those whose derived tag
/// is referenced verbatim by at least one job's `container.image`.
///
/// Two cases fall out of matching on the exact string, with no special-casing:
///
/// - An app directory nothing references is skipped. `workflow-023` ships two
///   `admet_pipeline` versions and uses one; the stale one is not built.
/// - A registry-qualified image is skipped, because a derived tag never carries
///   a registry prefix. `workflow-028` mixes
///   `ghcr.io/chiral-data/boltz:2025_09_05` with four local images, and only the
///   four are built.
pub fn apps_to_build(workflow_path: &Path, jobs: &[JobFolder]) -> Vec<LocalApp> {
    let referenced = referenced_images(jobs);

    discover_apps(workflow_path)
        .into_iter()
        .filter(|app| referenced.iter().any(|image| *image == app.image))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_app(base: &Path, dir_name: &str, with_dockerfile: bool) {
        let app_dir = base.join("apps").join(dir_name);
        fs::create_dir_all(&app_dir).unwrap();
        if with_dockerfile {
            fs::write(app_dir.join("Dockerfile"), "FROM scratch\n").unwrap();
        }
    }

    fn create_job(base: &Path, name: &str, image: &str) -> JobFolder {
        let job_dir = base.join(name);
        let chiral_dir = job_dir.join(".chiral");
        fs::create_dir_all(&chiral_dir).unwrap();
        fs::write(
            chiral_dir.join("job.toml"),
            format!(
                r#"name = "{name}"
description = "test"

[container]
image = "{image}"

[scripts]
run = "run.sh"
"#
            ),
        )
        .unwrap();
        JobFolder::new(name.to_string(), job_dir)
    }

    // --- tag derivation ---
    //
    // The expectations below are the tags the collab-workflows job configs
    // actually reference, checked against all ten app directories that exist.

    #[test]
    fn test_derive_single_word_name() {
        assert_eq!(
            derive_image_tag("p2rank_2026_07_10").unwrap(),
            "p2rank:2026_07_10"
        );
    }

    #[test]
    fn test_derive_multi_word_name_keeps_underscores() {
        // The case apps/build.sh gets wrong: it would yield admet:pipeline_2026_06_02
        assert_eq!(
            derive_image_tag("admet_pipeline_2026_06_02").unwrap(),
            "admet_pipeline:2026_06_02"
        );
        assert_eq!(
            derive_image_tag("solubility_pipeline_2026_06_05").unwrap(),
            "solubility_pipeline:2026_06_05"
        );
        assert_eq!(
            derive_image_tag("binder_design_2026_06_28").unwrap(),
            "binder_design:2026_06_28"
        );
        assert_eq!(
            derive_image_tag("polymer_md_2026_07_28").unwrap(),
            "polymer_md:2026_07_28"
        );
        assert_eq!(
            derive_image_tag("barrier_films_2026_07_28").unwrap(),
            "barrier_films:2026_07_28"
        );
    }

    #[test]
    fn test_derive_hyphenated_name() {
        assert_eq!(
            derive_image_tag("pipeline-report_2026_07_10").unwrap(),
            "pipeline-report:2026_07_10"
        );
        assert_eq!(
            derive_image_tag("unimol-docking_2026_07_10").unwrap(),
            "unimol-docking:2026_07_10"
        );
        assert_eq!(
            derive_image_tag("pocket-qc_2026_07_10").unwrap(),
            "pocket-qc:2026_07_10"
        );
    }

    #[test]
    fn test_derive_version_suffix_stays_in_tag() {
        assert_eq!(
            derive_image_tag("chai_2026_05_20_v2").unwrap(),
            "chai:2026_05_20_v2"
        );
    }

    #[test]
    fn test_derive_date_parts_do_not_create_a_boundary() {
        // `_05_` and `_31` are not four digits, so the only boundary is before 2026.
        assert_eq!(
            derive_image_tag("admet_pipeline_2026_05_31").unwrap(),
            "admet_pipeline:2026_05_31"
        );
    }

    #[test]
    fn test_derive_uses_the_last_boundary() {
        assert_eq!(
            derive_image_tag("run_2020_rerun_2026_01_01").unwrap(),
            "run_2020_rerun:2026_01_01"
        );
    }

    #[test]
    fn test_derive_rejects_names_without_a_date() {
        assert!(derive_image_tag("blast").is_none());
        assert!(derive_image_tag("blast_latest").is_none());
        assert!(derive_image_tag("blast_202").is_none());
        assert!(derive_image_tag("").is_none());
    }

    #[test]
    fn test_derive_rejects_empty_name_or_tag() {
        assert!(derive_image_tag("_2026_07_10").is_none());
        assert!(derive_image_tag("blast_").is_none());
    }

    // --- discovery ---

    #[test]
    fn test_discover_no_apps_folder_is_empty() {
        let temp = TempDir::new().unwrap();
        assert!(discover_apps(temp.path()).is_empty());
    }

    #[test]
    fn test_discover_finds_apps_with_dockerfile() {
        let temp = TempDir::new().unwrap();
        create_app(temp.path(), "p2rank_2026_07_10", true);
        create_app(temp.path(), "pocket-qc_2026_07_10", true);

        let apps = discover_apps(temp.path());
        assert_eq!(apps.len(), 2);
        // sorted by directory name
        assert_eq!(apps[0].image, "p2rank:2026_07_10");
        assert_eq!(apps[1].image, "pocket-qc:2026_07_10");
        assert!(apps[0].dockerfile.ends_with("p2rank_2026_07_10/Dockerfile"));
    }

    #[test]
    fn test_discover_skips_dirs_without_dockerfile() {
        let temp = TempDir::new().unwrap();
        create_app(temp.path(), "p2rank_2026_07_10", true);
        create_app(temp.path(), "notes_2026_07_10", false);

        let apps = discover_apps(temp.path());
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].image, "p2rank:2026_07_10");
    }

    #[test]
    fn test_discover_skips_undated_dirs() {
        let temp = TempDir::new().unwrap();
        create_app(temp.path(), "scratch", true);
        assert!(discover_apps(temp.path()).is_empty());
    }

    #[test]
    fn test_discover_does_not_recurse() {
        let temp = TempDir::new().unwrap();
        let nested = temp.path().join("apps/group_2026_07_10/inner_2026_07_10");
        fs::create_dir_all(&nested).unwrap();
        fs::write(nested.join("Dockerfile"), "FROM scratch\n").unwrap();

        // group_2026_07_10 itself has no Dockerfile, and the scan stops there.
        assert!(discover_apps(temp.path()).is_empty());
    }

    // --- referenced filtering ---

    #[test]
    fn test_apps_to_build_only_referenced() {
        let temp = TempDir::new().unwrap();
        create_app(temp.path(), "admet_pipeline_2026_05_31", true);
        create_app(temp.path(), "admet_pipeline_2026_06_02", true);
        let job = create_job(temp.path(), "01-admet", "admet_pipeline:2026_06_02");

        let to_build = apps_to_build(temp.path(), &[job]);
        assert_eq!(to_build.len(), 1);
        assert_eq!(to_build[0].image, "admet_pipeline:2026_06_02");
    }

    #[test]
    fn test_apps_to_build_skips_registry_qualified_images() {
        let temp = TempDir::new().unwrap();
        create_app(temp.path(), "p2rank_2026_07_10", true);
        let local = create_job(temp.path(), "01-pockets", "p2rank:2026_07_10");
        let remote = create_job(
            temp.path(),
            "02-fold",
            "ghcr.io/chiral-data/boltz:2025_09_05",
        );

        let to_build = apps_to_build(temp.path(), &[local, remote]);
        assert_eq!(to_build.len(), 1);
        assert_eq!(to_build[0].image, "p2rank:2026_07_10");
    }

    #[test]
    fn test_apps_to_build_empty_when_nothing_matches() {
        let temp = TempDir::new().unwrap();
        create_app(temp.path(), "p2rank_2026_07_10", true);
        let job = create_job(temp.path(), "01-other", "gromacs:2025_11_12");

        assert!(apps_to_build(temp.path(), &[job]).is_empty());
    }

    #[test]
    fn test_apps_to_build_empty_without_apps_folder() {
        let temp = TempDir::new().unwrap();
        let job = create_job(temp.path(), "01-only", "ubuntu:22.04");
        assert!(apps_to_build(temp.path(), &[job]).is_empty());
    }
}

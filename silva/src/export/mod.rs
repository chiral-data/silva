//! `silva export`: a workflow folder in another engine's language.
//!
//! [`ir::lower`] applies silva's rules once; each target emits from its result.

pub mod cwl;
pub mod ir;

use std::path::{Path, PathBuf};

/// Exports the workflow at `path` as CWL into `out` (default `<path>/cwl`).
/// Returns the folder written and the warnings to show.
pub fn export_cwl(
    path: &Path,
    out: Option<&Path>,
    force: bool,
) -> Result<(PathBuf, Vec<String>), String> {
    let workflow = ir::lower(path)?;
    let out = out
        .map(Path::to_path_buf)
        .unwrap_or_else(|| path.join("cwl"));
    let occupied = out
        .read_dir()
        .map(|mut entries| entries.next().is_some())
        .unwrap_or(false);
    if occupied && !force {
        return Err(format!(
            "{} already exists and is not empty; pass --force to overwrite it.",
            out.display()
        ));
    }
    std::fs::create_dir_all(&out).map_err(|e| format!("Cannot create {}: {e}", out.display()))?;
    let out = out
        .canonicalize()
        .map_err(|e| format!("Cannot resolve {}: {e}", out.display()))?;

    for (name, content) in cwl::emit(&workflow, &out)? {
        let file = out.join(&name);
        std::fs::write(&file, content)
            .map_err(|e| format!("Cannot write {}: {e}", file.display()))?;
    }
    Ok((out, workflow.warnings))
}

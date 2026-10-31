//! `silva export`: a workflow folder in another engine's language.
//!
//! [`ir::lower`] applies silva's rules once; each target emits from its result.

pub mod cwl;
mod driver;
pub mod ir;
pub mod wdl;

use std::path::{Path, PathBuf};

/// Exports the workflow at `path` as CWL into `out` (default `<path>/cwl`).
/// Returns the folder written and the warnings to show.
pub fn export_cwl(
    path: &Path,
    out: Option<&Path>,
    force: bool,
) -> Result<(PathBuf, Vec<String>), String> {
    let workflow = ir::lower(path)?;
    let out = prepare(path, out, "cwl", force)?;
    write(&out, cwl::emit(&workflow, &out)?)?;
    Ok((out, cwl::warnings(&workflow.caveats)))
}

/// Exports the workflow at `path` as WDL into `out` (default `<path>/wdl`).
/// Returns the folder written and the warnings to show.
pub fn export_wdl(
    path: &Path,
    out: Option<&Path>,
    force: bool,
) -> Result<(PathBuf, Vec<String>), String> {
    let workflow = ir::lower(path)?;
    // Emit before touching the folder: a refusal writes nothing.
    let files = wdl::emit(&workflow)?;
    let out = prepare(path, out, "wdl", force)?;
    write(&out, files)?;
    Ok((out, wdl::warnings(&workflow)))
}

/// The output folder, created and canonical; a non-empty one needs `force`.
fn prepare(path: &Path, out: Option<&Path>, default: &str, force: bool) -> Result<PathBuf, String> {
    let out = out
        .map(Path::to_path_buf)
        .unwrap_or_else(|| path.join(default));
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
    out.canonicalize()
        .map_err(|e| format!("Cannot resolve {}: {e}", out.display()))
}

fn write(out: &Path, files: Vec<(String, String)>) -> Result<(), String> {
    for (name, content) in files {
        let file = out.join(&name);
        std::fs::write(&file, content)
            .map_err(|e| format!("Cannot write {}: {e}", file.display()))?;
    }
    Ok(())
}

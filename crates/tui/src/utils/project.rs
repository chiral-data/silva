use std::path::PathBuf;

use crate::data_model;


pub fn dir(store: &data_model::Store) -> anyhow::Result<PathBuf> {
    let proj_dir = store.proj_selected.as_ref()
        .ok_or(anyhow::Error::msg("no project selected"))?;
    Ok(proj_dir.to_owned())
}

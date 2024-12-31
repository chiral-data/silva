use crate::data_model;
use crate::ui;

pub const HELPER: &[&str] = &[
    "Pre-processing", 
    "run the script @pre.sh", 
];

pub fn action(_states: &mut ui::states::States, store: &mut data_model::Store) -> anyhow::Result<()> {
    let proj_sel = store.project_sel.as_mut()
        .ok_or(anyhow::Error::msg("no selected project"))?;
    let proj_dir = proj_sel.dir.to_owned();

    let jh = tokio::spawn(async move {
        let _ = tokio::process::Command::new("sh")
            .current_dir(proj_dir)
            .arg("@pre.sh")
            .output().await.unwrap();
    });
    proj_sel.jh_pre = Some(jh);

    Ok(())
}


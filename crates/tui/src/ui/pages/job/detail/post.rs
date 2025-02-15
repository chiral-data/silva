use crate::data_model;
use crate::ui;

pub const HELPER: &[&str] = &[
    "post-processing", 
    "run the script @post.sh", 
];

pub fn action(_states: &mut ui::states::States, store: &mut data_model::Store) -> anyhow::Result<()> {
    let (proj_sel, proj_mgr) = store.project_sel.as_mut()
        .ok_or(anyhow::Error::msg("no selected project"))?;
    let _ = proj_sel.get_dir().join("@post.sh").exists().then_some(0)
        .ok_or(anyhow::Error::msg("script file @post.sh not exist"))?;
    let proj_dir = proj_sel.get_dir().to_owned();

    let jh = tokio::spawn(async move {
        let _ = tokio::process::Command::new("sh")
            .current_dir(proj_dir)
            .arg("@post.sh")
            .output().await.unwrap();
    });
    proj_mgr.add_post_processing(jh);

    Ok(())
}


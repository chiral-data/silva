use crate::{data_model, ui};


pub const HELPER: &[&str] = &[
    "Select the computaion infrastructure (Pod) for the job", 
];

pub fn action(states: &mut ui::states::States, _store: &data_model::Store) -> anyhow::Result<()> {
    states.job_states.show_page = ui::pages::job::ShowPage::AppList;
    Ok(())
}

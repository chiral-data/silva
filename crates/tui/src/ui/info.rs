//! Info Panel at the bottom

use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::ui;
use crate::data_model;

#[derive(Default)]
pub struct States {
    pub message: String 
}

pub fn render(f: &mut Frame, area: Rect, states: &ui::States, store: &data_model::Store) {
    let states_current = &states.info;

    let project_sel = if let Some(proj) = store.proj_selected.as_ref() {
        proj.to_str().unwrap()
    } else { "None" };
    let pod_type_sel_string = if let Some(pt) = states.infra.app_detail.pod_type_selected() {
       pt.name.to_string()
    } else { "None".to_string() };
    let pod_sel_string = if let Some(pod) = store.pod_mgr.selected() {
        pod.name.to_string()
    } else { "None".to_string() };

    let text: Vec<Line> = vec![
        Line::from(format!("[Selected Project]   {project_sel}")).green(),
        Line::from(format!("[Selected Pod Type]  {pod_type_sel_string}")).green(),
        Line::from(format!("[Selected Pod]       {pod_sel_string}")).green(),
        Line::from(format!("[Message] {}", states_current.message)).blue()
    ];

    let paragrah = Paragraph::new(text)
        .block(Block::default().title(" Info ").borders(Borders::ALL));

    f.render_widget(paragrah, area) 
}

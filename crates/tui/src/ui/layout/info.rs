//! Info Panel at the bottom

use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::ui;
use crate::data_model;

#[derive(Default)]
pub enum MessageLevel {
    #[default]
    Info,
    Warn,
    Error
}

impl MessageLevel {
    fn color(&self) -> Color {
        match &self {
            MessageLevel::Info => Color::Green,
            MessageLevel::Warn => Color::Yellow,
            MessageLevel::Error => Color::Red,
        }
    }
}

#[derive(Default)]
pub struct States {
    pub message: (String , MessageLevel)
}

pub fn render(f: &mut Frame, area: Rect, states: &ui::states::States, store: &mut data_model::Store) {
    let states_current = &states.info_states;

    let project_sel = if let Some(proj) = store.project_sel.as_ref() {
        proj.dir.to_str().unwrap()
    } else { "None" };
    let pod_type_sel_string = if let Some(pt) = states.job_states.app_detail.pod_type_selected() {
       pt.name.to_string()
    } else { "None".to_string() };
    let pod_sel_string = if let Some(pod) = store.pod_mgr.selected() {
        pod.name.to_string()
    } else { "None".to_string() };

    let mut text: Vec<Line> = vec![
        Line::from(format!("[Selected Project]   {project_sel}")).green(),
        Line::from(format!("[Selected Pod Type]  {pod_type_sel_string}")).green(),
        Line::from(format!("[Selected Pod]       {pod_sel_string}")).green(),
    ];
    if let Some(proj) = store.project_sel.as_mut() {
        if let Some(jh_pre) = proj.jh_pre.as_ref() {
            if jh_pre.is_finished() {
                proj.jh_pre = None;
            } else {
                text.push(Line::from("[Pre-processing] running ...").green());
            }
        }
        if let Some(jh_post) = proj.jh_post.as_ref() {
            if jh_post.is_finished() {
                proj.jh_post = None;
            } else {
                text.push(Line::from("[Post-processing] running ...").green());
            }
        }
    }
    let paragrah = Paragraph::new(text)
        .block(Block::default().title(" Info ").borders(Borders::ALL));

    if states_current.message.0.is_empty() {
        f.render_widget(paragrah, area);
    } else {
        let messages = vec![
            Line::from(format!("[Message] {}", states_current.message.0))
        ];
        let notification = Paragraph::new(messages)
            .style(Style::default().fg(states_current.message.1.color()))
            .block(Block::default().title(" Notification ").borders(Borders::ALL));

        let top_bottom = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Min(1)]) 
            .split(area);
        let (top, bottom) = (top_bottom[0], top_bottom[1]);
        f.render_widget(notification, top);
        f.render_widget(paragrah, bottom);
    }
}

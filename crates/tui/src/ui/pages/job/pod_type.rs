use ratatui::prelude::*;
use ratatui::widgets::*;
use crossterm::event;

use crate::data_model;
use crate::ui;

#[derive(Default)]
pub struct States {
    pub pod_type_sel_id: Option<usize>,
    pub pods: Vec<data_model::pod::Pod>,
    pub list_state_pods: ListState, 
}

pub fn render(f: &mut Frame, area: Rect, states: &mut ui::states::States, store: &mut data_model::Store) {
    let current_style = states.get_style(true);
    let states_current = &mut states.job_states.pod_type;

    let server_plan = store.pod_type_mgr.pod_types.get(&states_current.pod_type_sel_id.unwrap_or_default()).unwrap();
    let text: Vec<Line> = server_plan.descs.iter().map(|s| Line::from(s.as_str())).collect();
    let paragraph = Paragraph::new(text)
        .block(Block::bordered().title(server_plan.name.as_str()))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });

    let pt_selected = states.job_states.app_detail.pod_type_selected().unwrap(); 
    let actions = if pt_selected.is_service {
        vec![]
    } else {
        vec![
            Line::from("[C]eate server")
        ]
    };
    let action_list = Paragraph::new(actions)
        .block(Block::bordered().title(" Actions "))
        .style(current_style)
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });

    let pods_display: Vec<&str> = states_current.pods.iter().map(|p| p.name.as_str()).collect();
    let pod_list = List::new(pods_display)
        .block(Block::bordered().title(" Pods "))
        .style(current_style)
        .highlight_style(Style::new().reversed())
        .highlight_symbol(">>[Enter] ")
        .repeat_highlight_symbol(true)
        .direction(ListDirection::TopToBottom);
    if !states_current.pods.is_empty() && states_current.list_state_pods.selected().is_none() {
        states_current.list_state_pods.select(Some(0));
    }

    let top_mid_bottom = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Max(5), Constraint::Length(3), Constraint::Min(1)]) 
        .split(area);
    let (top, mid, bottom) = (top_mid_bottom[0], top_mid_bottom[1], top_mid_bottom[2]);

    f.render_widget(paragraph, top);
    f.render_widget(action_list, mid);
    f.render_stateful_widget(pod_list, bottom, &mut states_current.list_state_pods);
}

pub fn handle_key(key: &event::KeyEvent, states: &mut ui::states::States, store: &mut data_model::Store) {
    use event::KeyCode;
    let states_current = &mut states.job_states.pod_type;

    match key.code {
        KeyCode::Char('c') | KeyCode::Char('C') => {
            todo!()
            // let account_selected = store.account_mgr.selected(&store.setting_mgr)
            //     .ok_or(anyhow::Error::msg("no account selected"))
            //     .unwrap();
            // let client = account_selected.create_client();
            // let jh = tokio::spawn(async move {
            //     action::server::create_server(client).await.unwrap();
            // });

            // let total = store.job_mgr.jobs.len();
            // let mut job_local = data_model::job::Job::new(total, "create a server".to_string());
            // job_local.set_running();
            // let _ = states.handlers.insert(job_local.id, jh);
            // store.job_mgr.jobs.insert(job_local.id, job_local);
        }
        KeyCode::Up => {
            let total = states_current.pods.len(); 
            if total > 0 {
                let mut sel_idx = states_current.list_state_pods.selected().unwrap_or(0);
                sel_idx = (sel_idx + total - 1) % total; 
                states_current.list_state_pods.select(Some(sel_idx));
            }
        }
        KeyCode::Down => {
            let total = states_current.pods.len(); 
            if total > 0 {
                let mut sel_idx = states_current.list_state_pods.selected().unwrap_or(0);
                sel_idx = (sel_idx + 1) % total; 
                states_current.list_state_pods.select(Some(sel_idx));
            }
        }
        KeyCode::Enter => {
            if let Some(sel_idx) = states_current.list_state_pods.selected() {
                store.pod_mgr.pod_id_selected = Some(states_current.pods.get(sel_idx).unwrap().id);
                states.job_states.show_page = super::ShowPage::Detail;
            }
        }
        _ => ()
    }
}

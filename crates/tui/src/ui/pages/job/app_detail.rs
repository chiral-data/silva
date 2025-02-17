use ratatui::prelude::*;
use ratatui::widgets::*;
use crossterm::event;

use crate::data_model;
use crate::ui;
use crate::ui::layout::info::MessageLevel;

#[derive(Default)]
pub struct States {
    pub app: data_model::app::App,
    pub pod_types: Vec<data_model::pod_type::PodType>,
    pub list_state_pod_types: ListState,
}

impl States {
    pub fn pod_type_selected(&self) -> Option<&data_model::pod_type::PodType> {
        self.list_state_pod_types.selected()
            .map(|index_sel| self.pod_types.get(index_sel))?
    }
}

fn get_pod_types(states: &ui::states::States) -> &[data_model::pod_type::PodType] {
    &states.job_states.app_detail.pod_types
}

pub fn render(f: &mut Frame, area: Rect, states: &mut ui::states::States, store: &data_model::Store) {
    let current_style = states.get_style(true);
    let app_sel = super::app_list::get_selected(states, store).unwrap();
    let text = vec![
        Line::from("A free and open-source software suite for high-performance molecular dynamics and output analysis."),
        Line::from("https://www.gromacs.org".italic()),
    ];
    let paragraph = Paragraph::new(text)
        .block(Block::bordered().title(app_sel.as_str()))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });

    let server_plans_display = get_pod_types(states).iter()
            .map(|sp| sp.name.to_string())
            .collect::<Vec<String>>();

    let server_plan_list = List::new(server_plans_display)
        .block(Block::bordered().title(" Recommended Server Plans "))
        .style(current_style)
        .highlight_style(Style::new().reversed())
        .highlight_symbol(">>[Enter] ")
        .repeat_highlight_symbol(true)
        .direction(ListDirection::TopToBottom);
    if states.job_states.app_detail.list_state_pod_types.selected().is_none() {
        states.job_states.app_detail.list_state_pod_types.select(Some(0));
    }

    let top_bottom = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Max(5), Constraint::Min(1)]) 
        .split(area);
    let (top, bottom) = (top_bottom[0], top_bottom[1]);

    f.render_widget(paragraph, top);
    let list_state = &mut states.job_states.app_detail.list_state_pod_types;
    f.render_stateful_widget(server_plan_list, bottom, list_state);
}

fn select_pod_type(states: &mut ui::states::States, store: &mut data_model::Store, is_up: bool) {
    let states_current = &mut states.job_states.app_detail;
    let total = states_current.pod_types.len(); 
    let mut sel_idx = states_current.list_state_pod_types.selected().unwrap_or(0);
    if is_up {
        sel_idx = (sel_idx + total - 1) % total; 
    } else {
        sel_idx = (sel_idx + 1) % total; 
    }
    states_current.list_state_pod_types.select(Some(sel_idx));
    let pod_type = states_current.pod_types.get(sel_idx).unwrap();
    store.pod_type_mgr.pod_type_id_selected = Some(pod_type.id);
}

pub fn handle_key(key: &event::KeyEvent, states: &mut ui::states::States, store: &mut data_model::Store) {
    use event::KeyCode;
    let states_current = &mut states.job_states.app_detail;

    match key.code {
        KeyCode::Up => select_pod_type(states, store, true),
        KeyCode::Down => select_pod_type(states, store, false),
        KeyCode::Enter => {
            let list_state = &states_current.list_state_pod_types;
            if let Some(sel_idx) = list_state.selected() {
                let pod_type_sel = get_pod_types(states).get(sel_idx).unwrap().to_owned();

                if pod_type_sel.is_localhost() {
                    let localhost_pod_id = 0;
                    store.pod_mgr.pod_id_selected = Some(localhost_pod_id);
                    states.job_states.show_page = super::ShowPage::Detail;
                } else {
                    states.job_states.show_page = super::ShowPage::PodType;
                }

                let pods_of_this_type = store.pod_mgr.pods.values()
                    .filter(|pod| pod.type_id == pod_type_sel.id)
                    .map(|sv| sv.to_owned())
                    .collect::<Vec<data_model::pod::Pod>>();
                states.job_states.pod_type.pods = pods_of_this_type;
                store.pod_type_mgr.pod_type_id_selected = Some(pod_type_sel.id);
            } else {
                states.info_states.message = ("no server plan selected".to_string(), MessageLevel::Warn)
            }
        }
        KeyCode::Esc => states.job_states.show_page = super::ShowPage::AppList,
        _ => ()
    }
}

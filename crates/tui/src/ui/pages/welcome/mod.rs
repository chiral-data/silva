use ratatui::widgets::*;
use crate::ui::components as _components;

#[derive(Default)]
pub struct States {
    pub health_check_widget: _components::health_check::HealthCheck,
    pub health_check_state: ListState,
}


pub fn render(
    f: &mut ratatui::prelude::Frame,
    area: ratatui::prelude::Rect,
    states: &mut crate::ui::states::States,
    _store: &crate::data_model::Store,
) {
    let states_current = &mut states.welcome_states;
    states_current.health_check_widget.initialize();
    f.render_stateful_widget(states_current.health_check_widget.to_owned(), area, &mut states_current.health_check_state);
}

pub async fn handle_key(
    _key: &crossterm::event::KeyEvent,
    _states: &mut crate::ui::states::States,
    _store: &mut crate::data_model::Store,
) {
    unimplemented!()
}

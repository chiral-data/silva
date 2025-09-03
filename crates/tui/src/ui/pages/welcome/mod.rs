use ratatui::widgets::*;

mod health_check;


pub fn render(
    f: &mut ratatui::prelude::Frame,
    area: ratatui::prelude::Rect,
    _states: &mut crate::ui::states::States,
    _store: &crate::data_model::Store,
) {
    let mut hc = health_check::HealthCheck::new("Health Check Results");
    hc.add_item("Configuration loaded", health_check::ItemStatus::Success);
    hc.add_item("Gemini API connected", health_check::ItemStatus::Success);
    hc.add_item(
        "Local cache initialized",
        health_check::ItemStatus::Failure(Some("Permission denied".to_string())),
    );
    hc.add_item("Checking for updates", health_check::ItemStatus::Pending);
    let mut state = ListState::default();
    f.render_stateful_widget(hc, area, &mut state);
}

pub async fn handle_key(
    _key: &crossterm::event::KeyEvent,
    _states: &mut crate::ui::states::States,
    _store: &mut crate::data_model::Store,
) {
    unimplemented!()
}

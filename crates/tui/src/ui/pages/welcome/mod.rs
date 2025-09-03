mod health_check;


pub fn render(
    f: &mut ratatui::prelude::Frame,
    area: ratatui::prelude::Rect,
    _states: &mut crate::ui::states::States,
    _store: &crate::data_model::Store,
) {
    let mut page = WelcomePage::new("Welcome to gmn.nvim!");
    page.add_item("Configuration loaded", ItemStatus::Success);
    page.add_item("Gemini API connected", ItemStatus::Success);
    page.add_item(
        "Local cache initialized",
        ItemStatus::Failure(Some("Permission denied".to_string())),
    );
    page.add_item("Checking for updates", ItemStatus::Pending);
    let mut state = ListState::default();
    f.render_stateful_widget(page, area, &mut state);
}

pub async fn handle_key(
    _key: &crossterm::event::KeyEvent,
    _states: &mut crate::ui::states::States,
    _store: &mut crate::data_model::Store,
) {
    unimplemented!()
}

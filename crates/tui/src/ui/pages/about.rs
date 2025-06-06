use ratatui::prelude::*;
use ratatui::widgets::*;


pub fn render(f: &mut ratatui::prelude::Frame, area: ratatui::prelude::Rect, states: &mut crate::ui::states::States, _store: &crate::data_model::Store) {
    let current_style = states.get_style(true);
    let cargo_version = env!("CARGO_PKG_VERSION");
    let text = vec![
        format!("chiral silva version {}", cargo_version).into(),
    ];
    let par = Paragraph::new(text)
        .block(Block::bordered().padding(Padding::horizontal(1)))
        .style(current_style)
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });
    f.render_widget(par, area);
}

pub async fn handle_key(
    _key: &crossterm::event::KeyEvent,
    _states: &mut crate::ui::states::States,
    _store: &mut crate::data_model::Store
) {
    unimplemented!()
}




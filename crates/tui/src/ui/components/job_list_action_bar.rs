use ratatui::prelude::*;
use ratatui::widgets::*;

pub fn render(
    f: &mut Frame, area: Rect, 
    current_style: Style, action_selected: usize
) {
    let tabs_strings: Vec<String> = [
            ("New", "[N]ew"),
            ("Chat", "[C]ew"),
        ].into_iter()
        .enumerate()
        .map(|(i, s)| if i == action_selected {
            format!("[Enter] {}", s.0)
        } else { s.1.to_string() })
        .collect();
    let actions = Tabs::new(tabs_strings)
        .block(Block::bordered().title(" Actions "))
        .select(action_selected)
        .style(current_style);

    f.render_widget(actions, area);
}

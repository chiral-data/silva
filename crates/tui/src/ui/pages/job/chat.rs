//! Input adapted from the example: https://ratatui.rs/examples/apps/user_input/

use ratatui::prelude::*;
use ratatui::widgets::*;
use crossterm::event;

use crate::data_model;
use crate::ui;

#[derive(Default)]
pub struct States {
    messages: Vec<String>, 
    input: String,
    character_index: usize
}

impl States {
    fn byte_index(&self) -> usize {
        self.input
            .char_indices()
            .map(|(i, _)| i)
            .nth(self.character_index)
            .unwrap_or(self.input.len())
    }

    fn enter_char(&mut self, new_char: char) {
        let index = self.byte_index();
        self.input.insert(index, new_char);
        self.move_cursor_right();
    }

    fn move_cursor_left(&mut self) {
        let cursor_moved_left = self.character_index.saturating_sub(1);
        self.character_index = self.clamp_cursor(cursor_moved_left);
    }

    fn move_cursor_right(&mut self) {
        let cursor_moved_right = self.character_index.saturating_add(1);
        self.character_index = self.clamp_cursor(cursor_moved_right);
    }

    fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.input.chars().count())
    }

    fn delete_char(&mut self) {
        let is_not_cursor_leftmost = self.character_index != 0;
        if is_not_cursor_leftmost {
            let current_index = self.character_index;
            let from_left_to_current_index = current_index - 1;
            let before_char_to_delete = self.input.chars().take(from_left_to_current_index);
            let after_char_to_delete = self.input.chars().skip(current_index);
            self.input = before_char_to_delete.chain(after_char_to_delete).collect();
            self.move_cursor_left();
        }
    }

    fn submit(&mut self) {
        self.messages.push(self.input.clone());
        self.input.clear();
        self.character_index = 0;
    }
}

pub fn render(f: &mut Frame, area: Rect, states: &mut ui::states::States, _store: &mut data_model::Store) {
    let states_current = &mut states.job_states.chat;

    let vertical = Layout::vertical([
        Constraint::Length(area.height - 1),
        Constraint::Length(1),
    ]);
    let [message_area, input_area] = vertical.areas(f.area());
    let input = Paragraph::new(format!(">>>{}", states_current.input.as_str()))
        .style(Style::default().fg(Color::Yellow));
    f.render_widget(input, input_area);
    f.set_cursor_position(Position::new(
        input_area.x + states_current.character_index as u16 + 3,
        input_area.y,
    ));
    let messages: Vec<ListItem> = states_current.messages 
        .iter().rev()
        .map(|m| ListItem::new(m.as_str()))
        .collect();
    let messages = List::new(messages)
        .direction(ListDirection::BottomToTop);
    f.render_widget(messages, message_area);
}

pub fn handle_key(key: &event::KeyEvent, states: &mut ui::states::States, _store: &mut data_model::Store) {
    use event::KeyCode;
    let states_current = &mut states.job_states.chat;

    match key.code {
        KeyCode::Enter => states_current.submit(),
        KeyCode::Char(to_insert) => states_current.enter_char(to_insert),
        KeyCode::Backspace => states_current.delete_char(),
        KeyCode::Left => states_current.move_cursor_left(),
        KeyCode::Right => states_current.move_cursor_right(),
        _ => {}
    }
}

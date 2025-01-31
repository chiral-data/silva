//! Input adapted from the example: https://ratatui.rs/examples/apps/user_input/

use std::sync::{Arc, Mutex};

use ratatui::prelude::*;
use ratatui::widgets::*;
use crossterm::event;

use crate::data_model;
use crate::ui;


async fn ollama_generate(prompt: String, job_mgr: Arc<Mutex<data_model::job::Manager>>) {
    use ollama_rs::generation::completion::request::GenerationRequest;
    // use ollama_rs::generation::chat::{request::ChatMessageRequest, ChatMessageResponseStream};
    use tokio_stream::StreamExt;

    use ollama_rs::Ollama;
    let ollama = Ollama::new("http://100.98.250.114".to_string(), 11434);

    let model = "deepseek-r1:1.5b".to_string();
    let mut stream = ollama.generate_stream(GenerationRequest::new(model, prompt)).await.unwrap();

    while let Some(res) = stream.next().await {
        let responses = res.unwrap();
        for resp in responses {
            let mut job_mgr = job_mgr.lock().unwrap();
            job_mgr.chat_stream.push_str(resp.response.as_str());
        }
    }

}

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

    fn submit(&mut self) -> String {
        self.messages.push(self.input.clone());
        let prompt = self.input.clone();
        self.input.clear();
        self.character_index = 0;
        prompt
    }
}

pub fn render(f: &mut Frame, area: Rect, states: &mut ui::states::States, store: &mut data_model::Store) {
    let states_current = &mut states.job_states.chat;

    let mut job_mgr = store.job_mgr.lock().unwrap();
    if !job_mgr.chat_stream.is_empty() {
        let reply: String = job_mgr.chat_stream.drain(..).collect();
        let last_msg = if let Some(mut last_msg) = states_current.messages.pop() {
            last_msg.push_str(reply.as_str());
            last_msg
        } else {
            reply
        };
        states_current.messages.push(last_msg);
    }

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
        .map(|m| {
            let options = textwrap::Options::new(message_area.width as usize);
            let text = Text::from(
                textwrap::wrap(m, options)
                    .iter()
                    .map(|s| Line::from(s.to_string()))
                    .collect::<>()
                );
              ListItem::new(text)
        })
        .collect();
    let messages = List::new(messages)
        .direction(ListDirection::BottomToTop);
    f.render_widget(messages, message_area);
}

pub fn handle_key(key: &event::KeyEvent, states: &mut ui::states::States, store: &mut data_model::Store) {
    use event::KeyCode;
    let states_current = &mut states.job_states.chat;

    match key.code {
        KeyCode::Enter => {
            let prompt = states_current.submit();
            let job_mgr = store.job_mgr.clone();
            tokio::spawn(async move {
                ollama_generate(prompt, job_mgr.clone()).await 
            });
        }
        KeyCode::Char(to_insert) => states_current.enter_char(to_insert),
        KeyCode::Backspace => states_current.delete_char(),
        KeyCode::Left => states_current.move_cursor_left(),
        KeyCode::Right => states_current.move_cursor_right(),
        _ => {}
    }
}

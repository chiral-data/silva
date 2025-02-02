//! Input adapted from the example: https://ratatui.rs/examples/apps/user_input/

use std::sync::{Arc, Mutex};

use ratatui::prelude::*;
use ratatui::widgets::*;
use crossterm::event;

use crate::data_model;
use crate::ui;
use crate::ui::components;


async fn ollama_generate(prompt: String, job_mgr: Arc<Mutex<data_model::job::Manager>>) -> anyhow::Result<()> {
    use ollama_rs::generation::completion::request::GenerationRequest;
    use tokio_stream::StreamExt;

    use ollama_rs::Ollama;
    let ollama = Ollama::new("http://100.98.250.114".to_string(), 11434);

    let model = "deepseek-r1:1.5b".to_string();
    let mut stream = ollama.generate_stream(GenerationRequest::new(model, prompt)).await
        .map_err(|e| anyhow::Error::msg(format!("ollama generate stream error: {e}")))?;

    while let Some(res) = stream.next().await {
        let responses = res.map_err(|e| anyhow::Error::msg(format!("ollama stream response error: {e}")))?;
        for resp in responses {
            let mut job_mgr = job_mgr.lock().unwrap();
            job_mgr.chat_stream.push_str(resp.response.as_str());
        }
    }

    Ok(())
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
        self.messages.push(format!(">> {}", self.input));
        let prompt = self.input.clone();
        self.input.clear();
        self.character_index = 0;
        prompt
    }
}

pub fn render(f: &mut Frame, area: Rect, states: &mut ui::states::States, store: &mut data_model::Store) {
    let current_style = states.get_style(true);
    let states_current = &mut states.job_states.chat;

    let action_selected = match states.job_states.list.tab_action {
        super::list::Tab::New => 0,
        super::list::Tab::Chat => 1
    };

    let mut job_mgr = store.job_mgr.lock().unwrap();
    if !job_mgr.chat_stream.is_empty() && !states_current.messages.is_empty() {
        let reply: String = job_mgr.chat_stream.drain(..).collect();
        let last_msg = if states_current.messages.len() % 2 == 1 {
            reply
        } else {
            let mut last_msg = states_current.messages.pop().unwrap();
            last_msg.push_str(reply.as_str());
            last_msg
        };
        states_current.messages.push(last_msg);
    }

    let vertical = Layout::vertical([
        Constraint::Length(area.height - 1),
        Constraint::Length(1),
    ]);
    let top_mid_bottom = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Max(5), Constraint::Min(1)]) 
        .split(area);
    let (top, mid, bottom) = (top_mid_bottom[0], top_mid_bottom[1], top_mid_bottom[2]);
    let [message_area, input_area] = vertical.areas(bottom);

    let input = Paragraph::new(format!(">> {}", states_current.input.as_str()))
        .style(Style::default().fg(Color::Yellow));
    f.render_widget(input, input_area);
    f.set_cursor_position(Position::new(
        input_area.x + states_current.character_index as u16 + 3,
        input_area.y,
    ));
    let text: Vec<Line> = states_current.messages
        .iter()
        .rev()
        .take(message_area.height as usize)
        .rev()
        .flat_map(|s| tui_markdown::from_str(s))
        .collect();
    let line_count: u16 = text.iter()
        .map(|l| l.width() as u16 / message_area.width + 1)
        .sum();
    let messages = Paragraph::new(text)
        // .alignment(Alignment::Left)
        .wrap(Wrap { trim: true })
        .scroll((line_count.saturating_sub(message_area.height), 0));

    components::job_list_action_bar::render(f, top, current_style, action_selected);
    components::helper_hint::render_chat(f, mid, current_style);
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
                match ollama_generate(prompt, job_mgr.clone()).await {
                    Ok(()) => (),
                    Err(e) => {
                        let mut job_mgr = job_mgr.lock().unwrap();
                        job_mgr.add_log(0, format!("run job error: {e}"));
                    } 
                }
            });
        }
        KeyCode::Char(to_insert) => states_current.enter_char(to_insert),
        KeyCode::Backspace => states_current.delete_char(),
        KeyCode::Left => states_current.move_cursor_left(),
        KeyCode::Right => states_current.move_cursor_right(),
        _ => {}
    }
}

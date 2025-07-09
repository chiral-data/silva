//! Input adapted from the example: https://ratatui.rs/examples/apps/user_input/

use std::sync::{Arc, Mutex};

use ratatui::prelude::*;
use ratatui::widgets::*;
use crossterm::event;

use crate::data_model;
use crate::ui;

pub const HELPER: &[&str] = &[
    "Chat with the LLM after a LLM service launched",
    "For the 1st question, wait for a while for model loading"
];


async fn ollama_generate(prompt: String, job_mgr: Arc<Mutex<data_model::job::Manager>>) -> anyhow::Result<()> {
    use ollama_rs::generation::completion::request::GenerationRequest;
    use tokio_stream::StreamExt;

    // TODO: currently only support 1 job
    let job_id = 0;

    let http_uri = {
        let job_mgr = job_mgr.lock().unwrap();
        let job = job_mgr.jobs.get(&job_id)
            .ok_or(anyhow::Error::msg(format!("job {job_id} not found")))?;
        match &job.infra {
            data_model::job::Infra::None => None.ok_or(anyhow::Error::msg(format!("job {job_id} infra not ready")))?,
            data_model::job::Infra::Local => None.ok_or(anyhow::Error::msg(format!("job {job_id} for local infra not ready")))?,
            data_model::job::Infra::SakuraInternetDOK(_task_id, http_uri) => http_uri.clone().ok_or(anyhow::Error::msg("http uri not available yet"))?,
            data_model::job::Infra::RustClient(_task_id_id,_url)=>None.ok_or(anyhow::Error::msg(format!("job {job_id} infra not ready")))?,
        }
        // job_mgr.chat_stream.push_str(format!("responses from {http_uri}").as_str());
        // http_uri
    };

    use ollama_rs::Ollama;
    let ollama = Ollama::new(http_uri, 443);

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

fn render_service_available(f: &mut Frame, area: Rect, states: &mut ui::states::States, job_mgr: &mut data_model::job::Manager) {
    let states_current = &mut states.job_states.detail.chat;
    if !job_mgr.chat_stream.is_empty() && !states_current.messages.is_empty() {
        let reply: String = std::mem::take(&mut job_mgr.chat_stream);
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
    let [message_area, input_area] = vertical.areas(area);

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

    f.render_widget(messages, message_area);
}

fn render_service_unavailable(f: &mut Frame, area: Rect) {
    let vertical = Layout::vertical([
        Constraint::Length(area.height - 1),
        Constraint::Length(1),
    ]);
    let [message_area, input_area] = vertical.areas(area);

    let input = Paragraph::new("Service not available yet ...")
        .style(Style::default().fg(Color::Red));
    f.render_widget(input, input_area);
    let messages = Paragraph::new("")
        .wrap(Wrap { trim: true });

    f.render_widget(messages, message_area);
}

pub fn render(f: &mut Frame, area: Rect, states: &mut ui::states::States, store: &mut data_model::Store) {
    // TODO: currently only support one job
    let job_id = 0;

    let mut job_mgr = store.job_mgr.lock().unwrap();
    if let Some(job) = job_mgr.jobs.get(&job_id) {
        match &job.infra {
            data_model::job::Infra::SakuraInternetDOK(_, _) => render_service_available(f, area, states, &mut job_mgr),
            _ => render_service_unavailable(f, area)
        }
    } else {
        render_service_unavailable(f, area);
    }
}

pub fn handle_key(key: &event::KeyEvent, states: &mut ui::states::States, store: &mut data_model::Store) {
    use event::KeyCode;
    let states_current = &mut states.job_states.detail.chat;

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
        KeyCode::Esc => states.job_states.detail.tab_action = super::Tab::Files,
        _ => {}
    }
}

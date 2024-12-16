use std::time::{Duration, Instant};

use crossterm::event::{self, Event};
use ratatui::prelude::*;

use crate::data_model;

const COLOR_FOCUS: style::Color = style::Color::Yellow;

pub enum Signal {
    Quit,
    None
}

#[derive(Default, PartialEq)]
pub enum Focus {
    #[default]
    Tab,
    Main
}

pub fn render(f: &mut Frame, states: &mut states::States, store: &mut data_model::Store) {
    let area = f.area();  
    let top_mid_bottom = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1), Constraint::Length(7)]) 
        .split(area);
    let (top, mid, bottom) = (top_mid_bottom[0], top_mid_bottom[1], top_mid_bottom[2]);

    tabs::render(f, top, states);
    match states.tab.tab {
        tabs::Tab::Project => project::render(f, mid, states, store),
        tabs::Tab::Application => app::render(f, mid, states, store),
        tabs::Tab::Resource => resource::render(f, mid, states, store),
        tabs::Tab::Job => job::render(f, mid, states, store),
        tabs::Tab::Account => account::render(f, mid, states, store),
    }
    info::render(f, bottom, states, store)
}

pub async fn input(tick_rate: Duration, last_tick: &mut Instant, states: &mut states::States, store: &mut data_model::Store) -> anyhow::Result<Signal> {
    let timeout = tick_rate.checked_sub(last_tick.elapsed())
        .unwrap_or_else(|| Duration::from_secs(0));

    if event::poll(timeout)? {
        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Press {
                if key.modifiers == event::KeyModifiers::CONTROL && key.code == event::KeyCode::Char('q') {
                    return Ok(Signal::Quit);
                } else if key.code == event::KeyCode::Tab {
                    states.focus = Focus::Tab;
                    states.tab.tab = match states.tab.tab {
                        tabs::Tab::Project => tabs::Tab::Application,
                        tabs::Tab::Application => tabs::Tab::Resource, 
                        tabs::Tab::Resource => tabs::Tab::Job, 
                        tabs::Tab::Job => tabs::Tab::Account, 
                        tabs::Tab::Account => tabs::Tab::Project 
                    }
                } else {
                    match states.focus {
                        Focus::Tab => {
                            tabs::handle_key(&key, states);
                            states.focus = Focus::Main;
                        }
                        Focus::Main => match states.tab.tab {
                            tabs::Tab::Application => app::handle_key(&key, states, store),
                            tabs::Tab::Resource => resource::handle_key(&key, states, store),
                            tabs::Tab::Job => job::handle_key(&key, states, store),
                            tabs::Tab::Project => project::handle_key(&key, states, store),
                            tabs::Tab::Account => account::handle_key(&key, states, store) 
                        }
                    }
                }
            }
        }
    }

    if last_tick.elapsed() >= tick_rate {
        *last_tick = std::time::Instant::now();
    }

    Ok(Signal::None)
}

mod states;
pub use states::States;

// the top bar
mod tabs;
// the main body
mod app;
mod resource;
mod project;
mod job;
mod account;
// the footer
mod info;

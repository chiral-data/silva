use std::time::{Duration, Instant};

use crossterm::event;
use ratatui::prelude::*;

use crate::data_model;
use crate::ui;
use super::layout::*;
use super::pages::*;

pub enum Signal {
    Quit,
    None
}

pub fn render(f: &mut Frame, states: &mut ui::states::States, store: &mut data_model::Store) {
    let area = f.area();  
    let top_mid_bottom = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1), Constraint::Length(7)]) 
        .split(area);
    let (top, mid, bottom) = (top_mid_bottom[0], top_mid_bottom[1], top_mid_bottom[2]);

    tabs::render(f, top, states);
    match states.tabs_states.tab {
        tabs::Tab::Welcome => welcome::render(f, mid, states, store),
        tabs::Tab::Project => project::render(f, mid, states, store),
        tabs::Tab::Job => job::render(f, mid, states, store),
        tabs::Tab::Setting => setting::render(f, mid, states, store),
        tabs::Tab::About => about::render(f, mid, states, store),
    }
    info::render(f, bottom, states, store)
}

pub async fn handle_key(tick_rate: Duration, last_tick: &mut Instant, states: &mut ui::states::States, store: &mut data_model::Store) -> anyhow::Result<Signal> {
    let timeout = tick_rate.checked_sub(last_tick.elapsed())
        .unwrap_or_else(|| Duration::from_secs(0));

    if event::poll(timeout)? {
        if let event::Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Press {
                states.info_states.message.0.clear();

                if key.modifiers == event::KeyModifiers::CONTROL && key.code == event::KeyCode::Char('q') {
                    return Ok(Signal::Quit);
                // } else if key.modifiers == event::KeyModifiers::ALT && key.code == event::KeyCode::Tab {
                //     states.tab.tab = match states.tab.tab {
                //         tabs::Tab::Project => tabs::Tab::Setting,
                //         tabs::Tab::Infra => tabs::Tab::Project, 
                //         tabs::Tab::Job => tabs::Tab::Infra, 
                //         tabs::Tab::Setting => tabs::Tab::Job 
                //     };
                } else if key.code == event::KeyCode::Tab {
                    states.tabs_states.tab = match states.tabs_states.tab {
                        tabs::Tab::Welcome => tabs::Tab::Project,
                        tabs::Tab::Project => tabs::Tab::Job,
                        tabs::Tab::Job => tabs::Tab::Setting, 
                        tabs::Tab::Setting => tabs::Tab::About, 
                        tabs::Tab::About => tabs::Tab::Welcome
                    };
                } else {
                    match states.tabs_states.tab {
                        tabs::Tab::Welcome => welcome::handle_key(&key, states, store).await,
                        tabs::Tab::Project => project::handle_key(&key, states, store),
                        tabs::Tab::Job => job::handle_key(&key, states, store),
                        tabs::Tab::Setting => setting::handle_key(&key, states, store).await,
                        tabs::Tab::About => about::handle_key(&key, states, store).await 
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

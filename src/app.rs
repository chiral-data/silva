use std::io;

use crossterm::event::{KeyCode, KeyEvent};

use crate::{
    components::{application, health_check, workflow},
    layout,
};

pub struct App {
    pub selected_tab: usize,
    pub show_help: bool,
    pub footer_state: layout::footer::State,
    pub application_state: application::state::State,
    pub health_check_state: health_check::state::State,
    pub workflow_state: workflow::state::State,
}

impl App {
    pub async fn new() -> App {
        let loader = application::ApplicationLoader::default();
        let url = "https://raw.githubusercontent.com/chiral-data/container-images-silva/refs/heads/main/applications.json";
        let catalog = loader
            // .load_from_file()
            .load_with_fallback(Some(url)).await
            .unwrap_or_else(|_| application::ApplicationCatalog {
                version: String::from("1.0"),
                last_updated: String::new(),
                applications: Vec::new(),
            });

        App {
            selected_tab: 0,
            show_help: false,
            footer_state: layout::footer::State::default(),
            application_state: application::state::State::new(catalog),
            health_check_state: health_check::state::State::default(),
            workflow_state: workflow::state::State::default(),
        }
    }

    pub async fn update(&mut self) {
        self.footer_state.update();
        self.workflow_state.docker_state.update();
    }

    /// Handles keyboard input events.
    /// Returns Ok(true) if the app should quit, Ok(false) otherwise.
    pub async fn handle_key_event(&mut self, key: KeyEvent) -> io::Result<bool> {
        match key.code {
            // global keys, works everywhere
            KeyCode::Char('q') => return Ok(true),
            KeyCode::Char('i') => {
                self.show_help = !self.show_help;
            }
            _ => match key.code {
                KeyCode::Right | KeyCode::Char('l') => {
                    self.selected_tab = (self.selected_tab + 1) % 3
                }
                KeyCode::Left | KeyCode::Char('h') => {
                    self.selected_tab = if self.selected_tab > 0 {
                        self.selected_tab - 1
                    } else {
                        2
                    }
                }
                _ => {
                    if self.selected_tab == 0 {
                        self.application_state.handle_input(key);
                    } else if self.selected_tab == 1 {
                        self.workflow_state.handle_input(key).await;
                    } else if self.selected_tab == 2 {
                        self.health_check_state.handle_input(key);
                    }
                }
            },
        }

        Ok(false)
    }
}

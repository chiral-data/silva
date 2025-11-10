use crossterm::event::{KeyCode, KeyEvent};

use super::model::ApplicationCatalog;

pub struct State {
    pub catalog: ApplicationCatalog,
    pub selected_index: usize,
    pub show_popup: bool,
}

impl State {
    pub fn new(catalog: ApplicationCatalog) -> Self {
        Self {
            catalog,
            selected_index: 0,
            show_popup: false,
        }
    }

    pub fn selected_application(&self) -> Option<&super::model::Application> {
        self.catalog.applications.get(self.selected_index)
    }

    pub fn select_next(&mut self) {
        if !self.catalog.applications.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.catalog.applications.len();
        }
    }

    pub fn select_previous(&mut self) {
        if !self.catalog.applications.is_empty() {
            if self.selected_index == 0 {
                self.selected_index = self.catalog.applications.len() - 1;
            } else {
                self.selected_index -= 1;
            }
        }
    }

    pub fn toggle_popup(&mut self) {
        self.show_popup = !self.show_popup;
    }

    pub fn open_popup(&mut self) {
        self.show_popup = true;
    }

    pub fn close_popup(&mut self) {
        self.show_popup = false;
    }

    pub fn handle_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.select_previous(),
            KeyCode::Down | KeyCode::Char('j') => self.select_next(),
            KeyCode::Enter | KeyCode::Char('d') if !self.show_popup => self.open_popup(),
            KeyCode::Esc | KeyCode::Char('d') if self.show_popup => self.close_popup(),
            _ => {}
        }
    }
}

impl Default for State {
    fn default() -> Self {
        Self {
            catalog: ApplicationCatalog {
                version: String::new(),
                last_updated: String::new(),
                applications: Vec::new(),
            },
            selected_index: 0,
            show_popup: false,
        }
    }
}

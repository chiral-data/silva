use std::path::PathBuf;

use ratatui::prelude::*;
use ratatui::widgets::*;
use crossterm::event;

use crate::data_model;
use crate::ui;
use crate::ui::layout::info::MessageLevel;

#[derive(Default, PartialEq)]
pub enum Tab {
    #[default]
    Files,
    Build,
    Clear,
    Run
}

#[derive(Default)]
pub struct States {
    // job_settings: data_model::job::settings::Settings,
    tab_action: Tab,
    proj_dir: PathBuf,
    proj_files: Vec<String>,
    list_state_file: ListState,
}

// impl States {
//     pub fn update(&mut self, store: &data_model::Store) -> anyhow::Result<()> {
//         self.proj_dir = utils::project::dir(store)?;
//         self.job_settings = data_model::job::Job::get_settings(&self.proj_dir)?;
//         self.proj_files = self.job_settings.files.all_files();

//         Ok(())
//     }
// }

pub fn render(f: &mut Frame, area: Rect, states: &mut ui::states::States, store: &data_model::Store) {
    let current_style = states.get_style(true);
    let states_current = &mut states.job_states.detail;

    let action_selected = match states_current.tab_action {
        Tab::Files => 0,
        Tab::Build => 1,
        Tab::Clear => 2,
        Tab::Run => 3,
    };
    let tabs_strings: Vec<String> = [
            ("Files", "[F]iles"), 
            ("Build", "[B]lear"),
            ("Clear", "[C]lear"),
            ("Run", "[R]un"),
        ].into_iter()
        .enumerate()
        .map(|(i, s)| if i == action_selected {
            if i == 0 {
                s.0.to_string()
            } else { format!("[Enter] {}", s.0) }
        } else { s.1.to_string() })
        .collect();
    let actions = Tabs::new(tabs_strings)
        .block(Block::bordered().title(" Actions "))
        .select(action_selected)
        .divider(" ")
        .style(current_style);
    let helper_lines: Vec<Line> = match states_current.tab_action {
        Tab::Files => files::HELPER,
        Tab::Build => build::HELPER,
        Tab::Clear => clear::HELPER,
        Tab::Run => run::HELPER
    }.iter()
        .map(|&s| Line::from(s))
        .collect();
    let helper = Paragraph::new(helper_lines)
        .style(current_style)
        .block(Block::bordered())
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });

    let top_mid_bottom = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Max(5), Constraint::Min(1)]) 
        .split(area);
    let (top, mid, bottom) = (top_mid_bottom[0], top_mid_bottom[1], top_mid_bottom[2]);

    f.render_widget(actions, top);
    f.render_widget(helper, mid);
    match states_current.tab_action {
        Tab::Files => files::render(f, bottom, states, store),
        Tab::Build => (),
        Tab::Clear => (),
        Tab::Run => run::render(f, bottom, states, store)
    }
}

pub fn handle_key(key: &event::KeyEvent, states: &mut ui::states::States, store: &data_model::Store) {
    use event::KeyCode;

    let states_current = &mut states.job_states.detail;
    match key.code {
        KeyCode::Char('f') | KeyCode::Char('F') => states_current.tab_action = Tab::Files,
        KeyCode::Char('b') | KeyCode::Char('B') => states_current.tab_action = Tab::Build,
        KeyCode::Char('c') | KeyCode::Char('C') => states_current.tab_action = Tab::Clear,
        KeyCode::Char('r') | KeyCode::Char('R') => states_current.tab_action = Tab::Run,
        KeyCode::Enter => {
            match match states_current.tab_action {
                Tab::Files => Ok(()),
                Tab::Build => build::action(states, store),
                Tab::Clear => clear::action(states, store),
                Tab::Run => run::action(states, store)
            } {
                Ok(_) => (),
                Err(e) => states.info_states.message = (format!("job action error: {e}"), MessageLevel::Error),
            }
        }
        KeyCode::Esc => states.job_states.show_page = super::ShowPage::List,
        _ => {
            match states_current.tab_action {
                Tab::Files => files::handle_key(key, states, store),
                Tab::Build => (),
                Tab::Clear => (),
                Tab::Run => ()
            }
        }
    }
}

mod params;
mod files;
mod build;
mod clear;
mod run;

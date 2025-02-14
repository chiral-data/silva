use ratatui::prelude::*;
use ratatui::widgets::*;
use crossterm::event;

use crate::data_model;
use crate::ui;
use crate::ui::layout::info::MessageLevel;

#[derive(Default, PartialEq)]
pub enum Tab {
    Pod,
    #[default]
    Files,
    // Build,
    Pre,
    Run,
    Post,
}

impl Tab {
    fn texts(&self) -> (&str, &str) {
        match self {
            Self::Pod => ("Select a Pod", "[P]ods"), 
            Self::Files => ("Files", "[F]iles"), 
            // Self::Build => ("Build Docker Image", "[B]uild"),
            Self::Pre => ("Pre-processing", "Pr[e]"),
            Self::Run => ("Run", "[R]un"),
            Self::Post => ("Post-processing", "Po[s]t"),
        }
    }

    fn index(&self) -> usize {
        match self {
            Tab::Pod => 0,
            Tab::Files => 1,
            // Tab::Build => 2,
            Tab::Pre => 3,
            Tab::Run => 4,
            Tab::Post => 5,
        }
    }
}

#[derive(Default)]
pub struct States {
    // job_settings: data_model::job::settings::Settings,
    tab_action: Tab,
    list_state_file: ListState,
}

// impl States {
// }


// fn filter_tabs(tab: &Tab, states: &ui::states::States) -> bool {
//     match tab {
//         // build action not for localhost
//         Tab::Build => states.job_states.pod_type.pod_type_sel_id != Some(0),
//         _ => true
//     }
// }

pub fn render(f: &mut Frame, area: Rect, states: &mut ui::states::States, store: &data_model::Store) {
    let current_style = states.get_style(true);

    let tabs_strings: Vec<String> = [
            Tab::Pod, Tab::Files, Tab::Pre, Tab::Run, Tab::Post
        ].into_iter()
        // .filter(|t| filter_tabs(t, states))
        .map(|t| {
            let texts = t.texts();
            if t == states.job_states.detail.tab_action {
                if t == Tab::Files { texts.0.to_string() } else { format!("[Enter] {}", texts.0) }
            } else { texts.1.to_string() }
        })
        .collect();
    let states_current = &mut states.job_states.detail;
    let actions = Tabs::new(tabs_strings)
        .block(Block::bordered().title(" Actions "))
        .select(states_current.tab_action.index())
        .divider(" ")
        .style(current_style);
    let helper_lines: Vec<Line> = match states_current.tab_action {
        Tab::Pod => pod::HELPER, 
        Tab::Files => files::HELPER,
        // Tab::Build => build::HELPER,
        Tab::Pre => pre::HELPER,
        Tab::Run => run::HELPER,
        Tab::Post => post::HELPER,
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
        Tab::Pod => (),
        Tab::Files => files::render(f, bottom, states, store),
        // Tab::Build => (),
        Tab::Pre => (),
        Tab::Run => run::render(f, bottom, states, store),
        Tab::Post => (),
    }
}

pub fn handle_key(key: &event::KeyEvent, states: &mut ui::states::States, store: &mut data_model::Store) {
    use event::KeyCode;

    let states_current = &mut states.job_states.detail;
    match key.code {
        KeyCode::Char('p') | KeyCode::Char('P') => states_current.tab_action = Tab::Pod,
        KeyCode::Char('f') | KeyCode::Char('F') => states_current.tab_action = Tab::Files,
        // KeyCode::Char('b') | KeyCode::Char('B') => states_current.tab_action = Tab::Build,
        KeyCode::Char('e') | KeyCode::Char('E') => states_current.tab_action = Tab::Pre,
        KeyCode::Char('r') | KeyCode::Char('R') => states_current.tab_action = Tab::Run,
        KeyCode::Char('s') | KeyCode::Char('S') => states_current.tab_action = Tab::Post,
        KeyCode::Enter => {
            match match states_current.tab_action {
                Tab::Pod => pod::action(states, store),
                Tab::Files => Ok(()),
                // Tab::Build => build::action(states, store),
                Tab::Pre => pre::action(states, store),
                Tab::Run => run::action(states, store),
                Tab::Post => post::action(states, store),
            } {
                Ok(_) => (),
                Err(e) => states.update_info(format!("job action error: {e}"), MessageLevel::Error),
            }
        }
        KeyCode::Esc => states.job_states.show_page = super::ShowPage::List,
        _ => {
            match states_current.tab_action {
                Tab::Pod => (),
                Tab::Files => files::handle_key(key, states, store),
                // Tab::Build => (),
                Tab::Pre => (),
                Tab::Run => (),
                Tab::Post => (),
            }
        }
    }
}

mod params;
mod pod;
mod files;
// mod build; TODO
mod pre;
mod run;
mod post;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
};

use job_config::job::{JobMeta, ParamType};

use super::job::Job;

/// State for the parameter editor popup.
#[derive(Debug, Clone)]
pub struct ParamsEditorState {
    /// The job being edited
    pub job: Job,
    /// Job metadata with parameter definitions
    pub job_meta: JobMeta,
    /// Current parameter values (as strings for editing)
    pub param_values: Vec<(String, String)>,
    /// Currently selected parameter index
    pub selected_index: usize,
    /// Whether we're in edit mode for the selected parameter
    pub editing: bool,
    /// Input buffer for current edit
    pub input_buffer: String,
    /// Error message to display (if any)
    pub error_message: Option<String>,
}

impl ParamsEditorState {
    /// Creates a new parameter editor state for a job.
    pub fn new(job: Job, job_meta: JobMeta) -> Result<Self, String> {
        // Load current params or use defaults
        let current_params = job
            .load_params()
            .map_err(|e| format!("Failed to load params: {e}"))?
            .unwrap_or_else(|| job_meta.generate_default_params());

        // Convert params to editable strings
        let mut param_values = Vec::new();
        for (param_name, param_def) in &job_meta.params {
            let value = current_params
                .get(param_name)
                .unwrap_or(&param_def.default);
            let value_str = param_value_to_string(value);
            param_values.push((param_name.clone(), value_str));
        }

        // Sort by param name for consistent display
        param_values.sort_by(|a, b| a.0.cmp(&b.0));

        Ok(Self {
            job,
            job_meta,
            param_values,
            selected_index: 0,
            editing: false,
            input_buffer: String::new(),
            error_message: None,
        })
    }

    /// Moves selection up.
    pub fn move_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    /// Moves selection down.
    pub fn move_down(&mut self) {
        if self.selected_index < self.param_values.len().saturating_sub(1) {
            self.selected_index += 1;
        }
    }

    /// Starts editing the selected parameter.
    pub fn start_editing(&mut self) {
        if self.selected_index < self.param_values.len() {
            self.editing = true;
            self.input_buffer = self.param_values[self.selected_index].1.clone();
            self.error_message = None;
        }
    }

    /// Cancels the current edit.
    pub fn cancel_editing(&mut self) {
        self.editing = false;
        self.input_buffer.clear();
        self.error_message = None;
    }

    /// Saves the current edit.
    pub fn save_current_edit(&mut self) {
        if self.selected_index < self.param_values.len() {
            let param_name = &self.param_values[self.selected_index].0;

            // Validate the input
            if let Some(param_def) = self.job_meta.params.get(param_name) {
                match string_to_param_value(&self.input_buffer, &param_def.param_type) {
                    Ok(toml_value) => {
                        // Validate against the parameter definition
                        if let Err(e) = param_def.validate(&toml_value) {
                            self.error_message = Some(e);
                            return;
                        }

                        // Update the value
                        self.param_values[self.selected_index].1 = self.input_buffer.clone();
                        self.editing = false;
                        self.input_buffer.clear();
                        self.error_message = None;
                    }
                    Err(e) => {
                        self.error_message = Some(e);
                    }
                }
            }
        }
    }

    /// Adds a character to the input buffer.
    pub fn input_char(&mut self, c: char) {
        self.input_buffer.push(c);
    }

    /// Removes the last character from the input buffer.
    pub fn input_backspace(&mut self) {
        self.input_buffer.pop();
    }

    /// Saves all parameters to the job's params.toml file.
    pub fn save_params(&mut self) -> Result<(), String> {
        use std::collections::HashMap;

        let mut params: HashMap<String, toml::Value> = HashMap::new();

        for (param_name, param_value_str) in &self.param_values {
            if let Some(param_def) = self.job_meta.params.get(param_name) {
                let toml_value = string_to_param_value(param_value_str, &param_def.param_type)
                    .map_err(|e| format!("Invalid value for {param_name}: {e}"))?;

                param_def
                    .validate(&toml_value)
                    .map_err(|e| format!("Validation failed for {param_name}: {e}"))?;

                params.insert(param_name.clone(), toml_value);
            }
        }

        self.job
            .save_params(&params)
            .map_err(|e| format!("Failed to save params: {e}"))?;

        Ok(())
    }
}

/// Renders the parameter editor popup.
pub fn render(f: &mut Frame, state: &mut ParamsEditorState, area: Rect) {
    // Create centered popup area (60% width, 70% height)
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(15),
            Constraint::Percentage(70),
            Constraint::Percentage(15),
        ])
        .split(area);

    let popup_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(60),
            Constraint::Percentage(20),
        ])
        .split(popup_layout[1])[1];

    // Clear the background
    f.render_widget(Clear, popup_area);

    // Create the main popup block with background
    let title = format!(" Edit Parameters: {} ", state.job.name);
    let popup_block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Black));

    f.render_widget(popup_block, popup_area);

    // Get inner area for content (inside the borders)
    let inner_area = popup_area.inner(ratatui::layout::Margin {
        horizontal: 2,
        vertical: 1,
    });

    // Split into sections
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Description
            Constraint::Min(5),    // Parameters list
            Constraint::Length(3), // Error message / help
            Constraint::Length(1), // Controls hint
        ])
        .split(inner_area);

    // Render description
    let description = Paragraph::new(state.job_meta.description.as_str())
        .style(Style::default().fg(Color::Gray))
        .wrap(Wrap { trim: true });
    f.render_widget(description, sections[0]);

    // Render parameters list
    render_params_list(f, state, sections[1]);

    // Render error message or help text
    if let Some(error) = &state.error_message {
        let error_para = Paragraph::new(error.as_str())
            .style(Style::default().fg(Color::Red))
            .wrap(Wrap { trim: true });
        f.render_widget(error_para, sections[2]);
    } else if state.editing {
        let help = Paragraph::new("Type to edit | Enter: Save | Esc: Cancel")
            .style(Style::default().fg(Color::Yellow));
        f.render_widget(help, sections[2]);
    } else {
        let help = Paragraph::new("↑↓: Navigate | Enter: Edit | s: Save All | Esc: Cancel")
            .style(Style::default().fg(Color::Yellow));
        f.render_widget(help, sections[2]);
    }
}

fn render_params_list(f: &mut Frame, state: &ParamsEditorState, area: Rect) {
    let items: Vec<ListItem> = state
        .param_values
        .iter()
        .enumerate()
        .map(|(i, (name, value))| {
            let param_def = state.job_meta.params.get(name);
            let type_str = param_def
                .map(|d| d.param_type.to_string())
                .unwrap_or_else(|| "?".to_string());
            let hint = param_def.map(|d| d.hint.as_str()).unwrap_or("");

            let is_selected = i == state.selected_index;
            let is_editing = is_selected && state.editing;

            // Build the display text
            let display_value = if is_editing {
                state.input_buffer.to_string()
            } else {
                value.clone()
            };

            let mut lines = vec![];

            // First line: parameter name and value
            let name_style = if is_selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let value_style = if is_editing {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::UNDERLINED)
            } else if is_selected {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::Gray)
            };

            let cursor = if is_editing {
                "▶"
            } else if is_selected {
                ">"
            } else {
                " "
            };

            lines.push(Line::from(vec![
                Span::raw(cursor),
                Span::raw(" "),
                Span::styled(format!("{name} "), name_style),
                Span::raw("("),
                Span::styled(type_str, Style::default().fg(Color::Magenta)),
                Span::raw("): "),
                Span::styled(display_value, value_style),
            ]));

            // Second line: hint (if not editing or selected)
            if !hint.is_empty() && !is_editing {
                let hint_style = if is_selected {
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::ITALIC)
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                lines.push(Line::from(vec![
                    Span::raw("    "),
                    Span::styled(hint, hint_style),
                ]));
            }

            ListItem::new(lines)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::White))
            .title(" Parameters "),
    );

    f.render_widget(list, area);
}

/// Converts a TOML value to a displayable string.
fn param_value_to_string(value: &toml::Value) -> String {
    match value {
        toml::Value::String(s) => s.clone(),
        toml::Value::Integer(n) => n.to_string(),
        toml::Value::Float(f) => f.to_string(),
        toml::Value::Boolean(b) => b.to_string(),
        toml::Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(param_value_to_string).collect();
            format!("[{}]", items.join(", "))
        }
        toml::Value::Table(t) => format!("{:?}", t),
        toml::Value::Datetime(dt) => dt.to_string(),
    }
}

/// Converts a string to a TOML value based on the parameter type.
fn string_to_param_value(s: &str, param_type: &ParamType) -> Result<toml::Value, String> {
    let trimmed = s.trim();

    match param_type {
        ParamType::String | ParamType::File | ParamType::Directory | ParamType::Enum => {
            Ok(toml::Value::String(trimmed.to_string()))
        }
        ParamType::Integer => trimmed
            .parse::<i64>()
            .map(toml::Value::Integer)
            .map_err(|_| format!("Invalid integer: {trimmed}")),
        ParamType::Float => trimmed
            .parse::<f64>()
            .map(toml::Value::Float)
            .map_err(|_| format!("Invalid float: {trimmed}")),
        ParamType::Boolean => match trimmed.to_lowercase().as_str() {
            "true" | "yes" | "1" => Ok(toml::Value::Boolean(true)),
            "false" | "no" | "0" => Ok(toml::Value::Boolean(false)),
            _ => Err(format!("Invalid boolean: {trimmed} (use true/false)")),
        },
        ParamType::Array => {
            // Simple array parsing: split by comma
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                let inner = &trimmed[1..trimmed.len() - 1];
                let items: Vec<toml::Value> = inner
                    .split(',')
                    .map(|item| toml::Value::String(item.trim().to_string()))
                    .collect();
                Ok(toml::Value::Array(items))
            } else {
                // Also accept comma-separated without brackets
                let items: Vec<toml::Value> = trimmed
                    .split(',')
                    .map(|item| toml::Value::String(item.trim().to_string()))
                    .collect();
                Ok(toml::Value::Array(items))
            }
        }
    }
}

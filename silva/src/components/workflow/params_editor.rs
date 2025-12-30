use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
};

use job_config::job::ParamType;
use job_config::params::{JobParams, json_to_toml, toml_to_json};

use super::param_source::ParamSource;

/// State for the parameter editor popup.
/// Generic over the parameter source type.
#[derive(Debug, Clone)]
pub struct ParamsEditorState<T: ParamSource> {
    /// The source being edited (Job or WorkflowFolder with metadata)
    pub source: T,
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

impl<T: ParamSource> ParamsEditorState<T> {
    /// Creates a new parameter editor state from a param source.
    pub fn new(source: T) -> Result<Self, String> {
        // Load current params or use defaults
        let current_params = source
            .load_params()?
            .unwrap_or_else(|| source.generate_default_params());

        // Convert params to editable strings
        let mut param_values = Vec::new();
        for (param_name, param_def) in source.param_definitions() {
            // Get value from current params or convert default from TOML to JSON
            let default_json = toml_to_json(&param_def.default);
            let value = current_params.get(param_name).unwrap_or(&default_json);
            let value_str = param_value_to_string(value);
            param_values.push((param_name.clone(), value_str));
        }

        // Sort by param name for consistent display
        param_values.sort_by(|a, b| a.0.cmp(&b.0));

        Ok(Self {
            source,
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
            if let Some(param_def) = self.source.param_definitions().get(param_name) {
                match string_to_param_value(&self.input_buffer, &param_def.param_type) {
                    Ok(json_value) => {
                        // Convert JSON to TOML for validation against TOML-based ParamDefinition
                        let toml_value = json_to_toml(&json_value);
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

    /// Saves all parameters to the params file.
    pub fn save_params(&mut self) -> Result<(), String> {
        let mut params = JobParams::new();

        for (param_name, param_value_str) in &self.param_values {
            if let Some(param_def) = self.source.param_definitions().get(param_name) {
                let json_value = string_to_param_value(param_value_str, &param_def.param_type)
                    .map_err(|e| format!("Invalid value for {param_name}: {e}"))?;

                // Convert JSON to TOML for validation against TOML-based ParamDefinition
                let toml_value = json_to_toml(&json_value);
                param_def
                    .validate(&toml_value)
                    .map_err(|e| format!("Validation failed for {param_name}: {e}"))?;

                params.insert(param_name.clone(), json_value);
            }
        }

        self.source.save_params(&params)
    }
}

/// Renders the parameter editor popup.
pub fn render<T: ParamSource>(f: &mut Frame, state: &mut ParamsEditorState<T>, area: Rect) {
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
    let title_prefix = if state.source.is_global() {
        "Edit Global Parameters"
    } else {
        "Edit Parameters"
    };
    let title = format!(" {}: {} ", title_prefix, state.source.display_name());
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
    let description = Paragraph::new(state.source.description())
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

fn render_params_list<T: ParamSource>(f: &mut Frame, state: &ParamsEditorState<T>, area: Rect) {
    let items: Vec<ListItem> = state
        .param_values
        .iter()
        .enumerate()
        .map(|(i, (name, value))| {
            let param_def = state.source.param_definitions().get(name);
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

    // Use different title based on whether this is global or job params
    let list_title = if state.source.is_global() {
        " Global Parameters "
    } else {
        " Parameters "
    };

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::White))
            .title(list_title),
    );

    f.render_widget(list, area);
}

/// Converts a JSON value to a displayable string.
fn param_value_to_string(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(param_value_to_string).collect();
            format!("[{}]", items.join(", "))
        }
        serde_json::Value::Object(obj) => format!("{:?}", obj),
    }
}

/// Converts a string to a JSON value based on the parameter type.
fn string_to_param_value(s: &str, param_type: &ParamType) -> Result<serde_json::Value, String> {
    let trimmed = s.trim();

    match param_type {
        ParamType::String | ParamType::File | ParamType::Directory | ParamType::Enum => {
            Ok(serde_json::Value::String(trimmed.to_string()))
        }
        ParamType::Integer => trimmed
            .parse::<i64>()
            .map(|n| serde_json::json!(n))
            .map_err(|_| format!("Invalid integer: {trimmed}")),
        ParamType::Float => trimmed
            .parse::<f64>()
            .map(|f| serde_json::json!(f))
            .map_err(|_| format!("Invalid float: {trimmed}")),
        ParamType::Boolean => match trimmed.to_lowercase().as_str() {
            "true" | "yes" | "1" => Ok(serde_json::Value::Bool(true)),
            "false" | "no" | "0" => Ok(serde_json::Value::Bool(false)),
            _ => Err(format!("Invalid boolean: {trimmed} (use true/false)")),
        },
        ParamType::Array => {
            // Simple array parsing: split by comma
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                let inner = &trimmed[1..trimmed.len() - 1];
                let items: Vec<serde_json::Value> = inner
                    .split(',')
                    .map(|item| serde_json::Value::String(item.trim().to_string()))
                    .collect();
                Ok(serde_json::Value::Array(items))
            } else {
                // Also accept comma-separated without brackets
                let items: Vec<serde_json::Value> = trimmed
                    .split(',')
                    .map(|item| serde_json::Value::String(item.trim().to_string()))
                    .collect();
                Ok(serde_json::Value::Array(items))
            }
        }
    }
}

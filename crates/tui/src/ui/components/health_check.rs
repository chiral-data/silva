//! This module defines the `HealthCheck` component, a terminal UI component 
//! designed to display a dynamic list of items with their
//! respective statuses (e.g., success or failure).
//!
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::*,
};
use crate::action;

/// This component presents a dynamic list of items,
/// each indicating its status using visual cues like green checkmarks (✓) for success
/// or red crosses (✗) for failure. It's typically used to show initial setup steps,
/// feature availability, or recent activity summaries in a clear, at-a-glance format.
///
/// The `HealthCheck` is a `ratatui` `StatefulWidget`, meaning it manages
/// its own internal state, such as item list and scroll position.
///
/// # Structure
/// The `HealthCheck` comprises:
/// - A prominent title or welcome message at the top.
/// - A list of CheckItem`s, each with a description and status icon.
///
/// ```

#[derive(Debug, Default, Clone)]
pub struct HealthCheck {
    items: Vec<CheckItem>,
    is_initialized: bool,
}

impl HealthCheck {
    /// Adds a new item to the list.
    ///
    /// Each item consists of a description and a status, which determines
    /// whether a green checkmark, red cross, or other icon is displayed next to it.
    ///
    /// # Arguments
    /// * `description` - The textual description for the item.
    /// * `status` - The `ItemStatus` indicating success, failure, pending, or info.
    fn add_item(&mut self, description: impl Into<String>, status: ItemStatus) {
        self.items.push(CheckItem {
            description: description.into(),
            status,
        });
    }

    /// Updates the component.
    /// If the state is not yet initialized, it runs the initialization logic.
    /// Otherwise, it does nothing.
    pub fn initialize(&mut self) {
        if self.is_initialized {
            return;
        }

        match action::health_check::check_chiral_service() {
            Ok(msg) => {
                let new_msg = format!("[Chiral service OK] {msg} ");
                self.add_item(new_msg, ItemStatus::Success);
            }
            Err(e) => {
                self.add_item(
                    "[Chiral service Error] ",
                    ItemStatus::Failure(Some(e.to_string())),
                );
                match action::health_check::check_local_computer() {
                    Ok(msg) => {
                        let new_msg = format!("[Local computer as computation node OK] {msg}");
                        self.add_item(new_msg, ItemStatus::Success);
                    }
                    Err(e) => {
                        self.add_item(
                            "[Local computer as computation node ERROR] ",
                            ItemStatus::Failure(Some(e.to_string())),
                        );
                    }
                }
            }

        }

        self.is_initialized = true;
    }
}

impl StatefulWidget for HealthCheck {
    type State = ListState;
    /// Renders the `HealthCheck` widget onto the given `ratatui` buffer.
    ///
    /// This method draws the title, the block borders, and the list of items
    /// with their respective status icons and colors within the provided `Rect`.
    ///
    /// # Arguments
    /// * `area` - The `ratatui::layout::Rect` where the widget should be rendered.
    /// * `buffer` - A mutable reference to the `ratatui::buffer::Buffer` to draw on.
    /// * `state` - A mutable reference to the `ListState` to manage list selection and scrolling.
    fn render(self, area: Rect, buffer: &mut Buffer, state: &mut Self::State) {
        // Create a base block for the page
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Health Check Results ");
        let inner_area = block.inner(area);
        // Prepare list items
        let list_items: Vec<ListItem> = self.items.iter().map(|item| item.to_list_item()).collect();
        // Create the list widget
        let list = List::new(list_items)
            .block(Block::default().padding(Padding::new(2, 2, 0, 0))) // Use a default block for the list itself, as the parent block provides borders
            .highlight_style(Style::default().bg(Color::DarkGray)); // Highlight selected item
                                                                    // Render the list
        StatefulWidget::render(list, inner_area, buffer, state);
        // Render the parent block over the entire area
        block.render(area, buffer);
    }
}
/// Represents a single item displayed within the `HealthCheck` list.
///
/// Each item has a textual description and an associated status
/// that dictates its visual representation (e.g., icon and color).
#[derive(Debug, Clone)]
pub struct CheckItem {
    /// The descriptive text for this item.
    pub description: String,
    /// The status of the item, indicating success, failure, or other states.
    pub status: ItemStatus,
}

impl CheckItem {
    /// Converts the `CheckItem` into a `ratatui::widgets::ListItem` for rendering.
    ///
    /// The `ListItem` will include the status icon, the description, and
    /// appropriate styling based on the `ItemStatus`.
    ///
    /// # Returns
    /// A `ListItem` ready for display in a `ratatui` `List` widget.
    pub fn to_list_item(&self) -> ListItem<'_> {
        let icon = self.status.icon();
        let color = self.status.color();
        let status_text = if let ItemStatus::Failure(Some(msg)) = &self.status {
            format!("{} {} - {}", icon, self.description, msg)
        } else {
            format!("{} {}", icon, self.description)
        };
        ListItem::new(status_text).style(Style::default().fg(color))
    }
}

/// Defines the possible statuses for a `CheckItem`.
///
/// These statuses determine the icon and potentially the color
/// used when rendering the item in the terminal UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemStatus {
    /// Indicates a successful operation or state.
    Success,
    /// Indicates a failed operation or an error state.
    /// Optionally includes an error message for more detail.
    Failure(Option<String>),
    /// Indicates an ongoing or pending operation, often with an indeterminate state.
    #[allow(dead_code)]
    Pending,
}

impl ItemStatus {
    /// Returns the character icon associated with the status.
    ///
    /// * `Success`: '✓'
    /// * `Failure`: '✗'
    /// * `Pending`: '…'
    ///
    /// These icons are chosen for clear visual distinction in a terminal.
    pub fn icon(&self) -> char {
        match self {
            ItemStatus::Success => '✓',
            ItemStatus::Failure(_) => '✗',
            ItemStatus::Pending => '…',
        }
    }
    /// Returns the `ratatui::style::Color` associated with the status.
    ///
    /// * `Success`: `Color::Green`
    /// * `Failure`: `Color::Red`
    /// * `Pending`: `Color::Yellow`
    ///
    /// These colors provide immediate visual feedback on the item's state.
    pub fn color(&self) -> Color {
        match self {
            ItemStatus::Success => Color::Green,
            ItemStatus::Failure(_) => Color::Red,
            ItemStatus::Pending => Color::Yellow,
        }
    }
}



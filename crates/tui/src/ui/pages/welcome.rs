//! This module defines the `WelcomePage` component, a terminal UI page
//! designed to greet users and display a dynamic list of items with their
//! respective statuses (e.g., success or failure).
//!
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, StatefulWidget, Widget},
};

/// Represents the Welcome Page in the terminal user interface.
///
/// This page provides an initial greeting and presents a dynamic list of items,
/// each indicating its status using visual cues like green checkmarks (✓) for success
/// or red crosses (✗) for failure. It's typically used to show initial setup steps,
/// feature availability, or recent activity summaries in a clear, at-a-glance format.
///
/// The `WelcomePage` is a `ratatui` `StatefulWidget`, meaning it manages
/// its own internal state, such as item list and scroll position.
///
/// # Structure
/// The `WelcomePage` comprises:
/// - A prominent title or welcome message at the top.
/// - A scrollable list of `WelcomeItem`s, each with a description and status icon.
///
/// # Examples
/// ```rust
/// use ratatui::{backend::TestBackend, Terminal};
/// use ratatui::layout::Rect;
/// use gmn_nvim::ui::welcome_page::{WelcomePage, WelcomeItem, ItemStatus}; // Assuming this path
///
/// let mut backend = TestBackend::new(100, 20);
/// let mut terminal = Terminal::new(backend).unwrap();
///
/// let mut page = WelcomePage::new("Welcome to gmn.nvim!");
/// page.add_item("Configuration loaded", ItemStatus::Success);
/// page.add_item("Gemini API connected", ItemStatus::Success);
/// page.add_item("Local cache initialized", ItemStatus::Failure(Some("Permission denied".to_string())));
/// page.add_item("Checking for updates", ItemStatus::Pending);
///
/// // In a real application, you would render this within a main loop.
/// // terminal.draw(|f| {
/// //     let area = Rect::new(0,0,100,20);
/// //     f.render_stateful_widget(page, area, &mut page.state);
/// // }).unwrap();
/// ```

#[derive(Debug)]
pub struct WelcomePage {
    /// The main title or welcome message displayed at the top of the page.
    title: String,
    /// A vector of items to be displayed in the list.
    items: Vec<WelcomeItem>,
    /// The current state of the list widget, used for scrolling and selection.
    pub state: ListState,
}
impl WelcomePage {
    /// Creates a new `WelcomePage` with a specified title.
    ///
    /// Initializes the page with an empty list of items and a default
    /// `ListState` for managing the list's rendering behavior.
    ///
    /// # Arguments
    /// * `title` - A string slice that will be used as the main title for the page.
    ///
    /// # Returns
    /// A new `WelcomePage` instance.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            items: Vec::new(),
            state: ListState::default(),
        }
    }
    /// Adds a new item to the welcome page's list.
    ///
    /// Each item consists of a description and a status, which determines
    /// whether a green checkmark, red cross, or other icon is displayed next to it.
    ///
    /// # Arguments
    /// * `description` - The textual description for the item.
    /// * `status` - The `ItemStatus` indicating success, failure, pending, or info.
    pub fn add_item(&mut self, description: impl Into<String>, status: ItemStatus) {
        self.items.push(WelcomeItem {
            description: description.into(),
            status,
        });
    }
    /// Scrolls the list of items up by one position.
    ///
    /// If an item is currently selected, it moves the selection up.
    /// If at the top, it wraps around to the bottom.
    pub fn scroll_up(&mut self) {
        let i = match self.state.selected() {
            Some(selected) => {
                if selected == 0 {
                    self.items.len().saturating_sub(1)
                } else {
                    selected.saturating_sub(1)
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
    /// Scrolls the list of items down by one position.
    ///
    /// If an item is currently selected, it moves the selection down.
    /// If at the bottom, it wraps around to the top.
    pub fn scroll_down(&mut self) {
        let i = match self.state.selected() {
            Some(selected) => {
                if selected >= self.items.len().saturating_sub(1) {
                    0
                } else {
                    selected.saturating_add(1)
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
}
impl StatefulWidget for WelcomePage {
    type State = ListState;
    /// Renders the `WelcomePage` widget onto the given `ratatui` buffer.
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
            .title(self.title.as_str());
        let inner_area = block.inner(area);
        // Prepare list items
        let list_items: Vec<ListItem> = self
            .items
            .iter()
            .map(|item| item.to_list_item())
            .collect();
        // Create the list widget
        let list = List::new(list_items)
            .block(Block::default()) // Use a default block for the list itself, as the parent block provides borders
            .highlight_style(Style::default().bg(Color::DarkGray)); // Highlight selected item
        // Render the list
        StatefulWidget::render(list, inner_area, buffer, state);
        // Render the parent block over the entire area
        block.render(area, buffer);
    }
}
/// Represents a single item displayed within the `WelcomePage` list.
///
/// Each item has a textual description and an associated status
/// that dictates its visual representation (e.g., icon and color).
#[derive(Debug, Clone)]
pub struct WelcomeItem {
    /// The descriptive text for this item.
    pub description: String,
    /// The status of the item, indicating success, failure, or other states.
    pub status: ItemStatus,
}
impl WelcomeItem {
    /// Converts the `WelcomeItem` into a `ratatui::widgets::ListItem` for rendering.
    ///
    /// The `ListItem` will include the status icon, the description, and
    /// appropriate styling based on the `ItemStatus`.
    ///
    /// # Returns
    /// A `ListItem` ready for display in a `ratatui` `List` widget.
    pub fn to_list_item(&self) -> ListItem {
        let icon = self.status.icon();
        let color = self.status.color();
        let status_text = if let ItemStatus::Failure(Some(ref msg)) = self.status {
            format!("{} {} - {}", icon, self.description, msg)
        } else {
            format!("{} {}", icon, self.description)
        };
        ListItem::new(status_text).style(Style::default().fg(color))
    }
}
/// Defines the possible statuses for a `WelcomeItem`.
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
    Pending,
    /// A neutral or informational status, used for general messages.
    Info,
}
impl ItemStatus {
    /// Returns the character icon associated with the status.
    ///
    /// * `Success`: '✓'
    /// * `Failure`: '✗'
    /// * `Pending`: '…'
    /// * `Info`: 'i'
    ///
    /// These icons are chosen for clear visual distinction in a terminal.
    pub fn icon(&self) -> char {
        match self {
            ItemStatus::Success => '✓',
            ItemStatus::Failure(_) => '✗',
            ItemStatus::Pending => '…',
            ItemStatus::Info => 'i',
        }
    }
    /// Returns the `ratatui::style::Color` associated with the status.
    ///
    /// * `Success`: `Color::Green`
    /// * `Failure`: `Color::Red`
    /// * `Pending`: `Color::Yellow`
    /// * `Info`: `Color::LightBlue` (or `White` if `LightBlue` is too similar to `Success` on some terminals)
    ///
    /// These colors provide immediate visual feedback on the item's state.
    pub fn color(&self) -> Color {
        match self {
            ItemStatus::Success => Color::Green,
            ItemStatus::Failure(_) => Color::Red,
            ItemStatus::Pending => Color::Yellow,
            ItemStatus::Info => Color::LightBlue, // Or Color::White
        }
    }
}


use ratatui::prelude::*;
use ratatui::widgets::*;


pub fn render(f: &mut ratatui::prelude::Frame, area: ratatui::prelude::Rect, states: &mut crate::ui::states::States, _store: &crate::data_model::Store) {
    let current_style = states.get_style(true);
    let cargo_version = env!("CARGO_PKG_VERSION");
    let text = vec![
        format!("chiral silva version {}", cargo_version).into(),
    ];
    let par = Paragraph::new(text)
        .block(Block::bordered().padding(Padding::horizontal(1)))
        .style(current_style)
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });
    f.render_widget(par, area);
}

pub async fn handle_key(
    _key: &crossterm::event::KeyEvent,
    _states: &mut crate::ui::states::States,
    _store: &mut crate::data_model::Store
) {
    unimplemented!()
}




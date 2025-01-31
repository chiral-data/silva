use ratatui::prelude::*;
use ratatui::widgets::*;
use ratatui::style::Styled;
use crossterm::event;
use tui_scrollview::{ScrollView, ScrollViewState};

use crate::data_model;
use crate::ui;

#[derive(Debug, Default, Clone)]
struct ChatHistoryText {
    text: [String; 3],
    scroll_view_state: ScrollViewState,
    // state: AppState,
}

const SCROLLVIEW_HEIGHT: u16 = 100;

impl ratatui::prelude::Widget for &mut ChatHistoryText { 
    fn render(self, area: Rect, buf: &mut Buffer) {
        let layout = Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]);
        let [title, body] = layout.areas(area);

        // self.title().render(title, buf);
        let width = if buf.area.height < SCROLLVIEW_HEIGHT {
            buf.area.width - 1
        } else {
            buf.area.width
        };
        let mut scroll_view = ScrollView::new(Size::new(width, SCROLLVIEW_HEIGHT));
        self.render_widgets_into_scrollview(scroll_view.buf_mut());
        scroll_view.render(body, buf, &mut self.scroll_view_state)
    }
}

impl ChatHistoryText {
    fn new() -> Self {
        Self {
            text: [
                "hello".to_string(),
                "hello2".to_string(),
                "hello3".to_string()
            ],
            ..Default::default()
        }
    }

    fn render_widgets_into_scrollview(&self, buf: &mut Buffer) {
        use Constraint::*;
        let area = buf.area;
        let [numbers, widgets] = Layout::horizontal([Length(5), Fill(1)]).areas(area);
        let [bar_charts, text_0, text_1, text_2] =
            Layout::vertical([Length(7), Fill(1), Fill(2), Fill(4)]).areas(widgets);
        let [left_bar, right_bar] = Layout::horizontal([Length(20), Fill(1)]).areas(bar_charts);

        self.line_numbers(area.height).render(numbers, buf);
        self.text(0).render(text_0, buf);
        self.text(1).render(text_1, buf);
        self.text(2).render(text_2, buf);
    }

    fn line_numbers(&self, height: u16) -> impl Widget {
        use std::fmt::Write;
        let line_numbers = (1..=height).fold(String::new(), |mut output, n| {
            let _ = writeln!(output, "{n:>4} ");
            output
        });
        Text::from(line_numbers).dim()
    }

    fn text(&self, index: usize) -> impl Widget {
        let block = Block::bordered().title(format!("Text {}", index));
        Paragraph::new(self.text[index].clone())
            .wrap(Wrap { trim: false })
            .block(block)
    }
}

struct MyScrollableWidget;

impl StatefulWidget for MyScrollableWidget {
    type State = ScrollViewState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        // 100 lines of text
        let line_numbers = (1..=100).map(|i| format!("{:>3} ", i)).collect::<String>();
        let content =
            std::iter::repeat("Lorem ipsum dolor sit amet, consectetur adipiscing elit.\n")
                .take(100)
                .collect::<String>();

        let content_size = Size::new(100, 40);
        let mut scroll_view = ScrollView::new(content_size);

        // the layout doesn't have to be hardcoded like this, this is just an example
        scroll_view.render_widget(Paragraph::new(line_numbers), Rect::new(0, 0, 5, 100));
        scroll_view.render_widget(Paragraph::new(content), Rect::new(5, 0, 95, 100));

        scroll_view.render(buf.area, buf, state);
    }
}

pub fn render(f: &mut Frame, area: Rect, states: &mut ui::states::States, store: &mut data_model::Store) {
    let msw = MyScrollableWidget {};
    let mut state = ScrollViewState::new();
    f.render_stateful_widget(msw, area, &mut state);
}

use crossterm::event::Event;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph, StatefulWidget, Widget};
use std::borrow::Cow;
use std::ops::Deref;
use tui_input::Input;
use tui_input::backend::crossterm::EventHandler;

pub struct InputViewState<'a> {
    prefix: Cow<'a, str>,
    input: Input,
    cursor_pos: (u16, u16),
}

impl<'a> InputViewState<'a> {
    pub fn new() -> Self {
        Self {
            prefix: Cow::Borrowed(""),
            input: Input::new(String::new()),
            cursor_pos: (0, 0),
        }
    }

    #[inline]
    pub fn cursor(&self) -> (u16, u16) {
        self.cursor_pos
    }

    #[inline]
    pub fn prefix(&self) -> &str {
        self.prefix.as_ref()
    }

    #[inline]
    pub fn set_prefix(&mut self, prefix: &'a str) {
        self.prefix = Cow::Borrowed(prefix)
    }

    #[inline]
    pub fn input_value(&'a self) -> &'a str {
        self.input.value()
    }

    #[inline]
    pub fn reset(&mut self) {
        self.cursor_pos = (0, 0);
        self.input.reset();
    }

    #[inline]
    pub fn handle_event(&mut self, event: &Event) {
        self.input.handle_event(event);
    }

    #[inline]
    fn count_prefix_size(&self) -> usize {
        self.prefix.chars().fold(
            0usize,
            |acc, c| if c.is_ascii() { acc + 1 } else { acc + 2 },
        )
    }
}

pub struct InputView;

impl InputView {
    pub fn new() -> Self {
        Self
    }
}

impl StatefulWidget for InputView {
    type State = InputViewState<'static>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let prefix = state.prefix.deref();
        let prefix_len = state.count_prefix_size().saturating_sub(1);

        let scroll = state.input.visual_scroll(area.width as usize - prefix_len);
        let paragraph = Paragraph::new(vec![Line::from(vec![
            Span::raw(prefix),
            Span::raw(state.input.value()),
        ])])
        .style(Style::default())
        .scroll((0, scroll as u16))
        .block(Block::default());

        paragraph.render(area, buf);

        let x = state.input.visual_cursor().max(scroll) - scroll + 1;
        state.cursor_pos = (area.x + prefix_len as u16 + x as u16, area.y);
    }
}

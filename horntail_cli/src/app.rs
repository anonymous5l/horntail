use crate::error::Result;
use crate::row::HorntailRow;
use crate::widget::horntail::{HorntailView, HorntailViewState};
use crate::widget::image::{ImageView, ImageViewState};
use crate::widget::input::{InputView, InputViewState};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use horntail::Primitive;
use ratatui::DefaultTerminal;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::widgets::{Block, BorderType};
use ratatui_image::picker::Picker;
use std::num::NonZeroUsize;
use std::rc::Rc;

pub fn serve(root: HorntailRow, disable_preview: bool) -> Result<()> {
    let terminal = ratatui::try_init()?;

    // get terminal window size not work on Windows.
    // https://docs.rs/crossterm/latest/crossterm/terminal/fn.window_size.html
    let windows_size = crossterm::terminal::window_size()?;

    Layout::init_cache(
        NonZeroUsize::new(((windows_size.columns + windows_size.rows) * 2) as usize).unwrap(),
    );

    let picker = if disable_preview {
        None
    } else {
        let font_size = (
            windows_size.width / windows_size.columns,
            windows_size.height / windows_size.rows,
        );
        Some(Picker::from_fontsize(font_size))
    };

    App {
        terminal,
        root,
        picker,
        state: AppState::Running,
        event_mode: EventMode::Normal,
    }
    .serve()
}

#[derive(Default)]
enum AppState {
    #[default]
    Init,
    Running,
    Stop,
}

enum EventMode {
    Normal,
    Input,
}

struct App {
    terminal: DefaultTerminal,
    root: HorntailRow,
    picker: Option<Picker>,
    state: AppState,
    event_mode: EventMode,
}

impl App {
    fn serve(mut self) -> Result<()> {
        let mut key_event: Option<KeyEvent> = None;
        let mut horntail_view_state = HorntailViewState::new(self.root)?;

        let preview_block_template = Block::bordered()
            .black()
            .border_type(BorderType::Rounded)
            .border_style(Style::new().light_blue())
            .title("Preview");

        let mut image_offset = 0;
        let mut image_view_state = self.picker.map(ImageViewState::new);
        let mut text_view_state = InputViewState::new();

        while let AppState::Running = self.state {
            self.terminal.draw(|f| {
                let layout = if image_offset != 0 {
                    Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)]).split(f.area())
                } else {
                    Layout::horizontal([Constraint::Fill(1)]).split(f.area())
                };

                f.buffer_mut().reset();

                let main_layout = if let EventMode::Input = self.event_mode {
                    Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).split(layout[0])
                } else {
                    Rc::<[Rect]>::from([layout[0]])
                };

                f.render_stateful_widget(
                    HorntailView::new(),
                    main_layout[0],
                    &mut horntail_view_state,
                );

                if let EventMode::Input = self.event_mode {
                    f.render_stateful_widget(
                        InputView::new(),
                        main_layout[1],
                        &mut text_view_state,
                    );
                    f.set_cursor_position(text_view_state.cursor())
                }

                if let Some(image_view_state) = image_view_state.as_mut() {
                    if image_offset != 0 {
                        let preview_block = preview_block_template.clone();
                        let image_view_area = preview_block.inner(layout[1]);
                        f.render_widget(preview_block, layout[1]);
                        f.render_stateful_widget(
                            ImageView::new(),
                            image_view_area,
                            image_view_state,
                        );
                    }
                }
            })?;

            let event = crossterm::event::read()?;
            if let Event::Key(ke) = event {
                match self.event_mode {
                    EventMode::Input => {
                        handle_input_event(
                            event,
                            &mut self.event_mode,
                            &mut horntail_view_state,
                            &mut text_view_state,
                        );
                    }
                    EventMode::Normal => {
                        handle_normal_event(
                            ke,
                            key_event.as_ref(),
                            &mut self.state,
                            &mut self.event_mode,
                            &mut horntail_view_state,
                            &mut text_view_state,
                        );
                    }
                };
                key_event.replace(ke);
            }

            handle_selection(
                &mut image_offset,
                image_view_state.as_mut(),
                &mut horntail_view_state,
            );
        }

        Ok(())
    }
}

const SEARCH_PREFIX_NEXT: &str = "/";
const SEARCH_PREFIX_PREV: &str = "?";

fn handle_selection(
    image_offset: &mut usize,
    image_view_state: Option<&mut ImageViewState>,
    horntail_view_state: &mut HorntailViewState,
) {
    let Some(image_view_state) = image_view_state else {
        return;
    };

    let path = horntail_view_state.selected_paths();
    horntail_view_state.set_title_suffix(path.to_str().unwrap());

    let row = horntail_view_state.selected();
    let Some(canvas) = row.to_canvas() else {
        *image_offset = 0;
        image_view_state.reset();
        return;
    };

    let offset = row.offset;
    if *image_offset == offset {
        return;
    }

    if let Some(prop) = canvas
        .attr
        .property
        .as_ref()
        .and_then(|x| x.iter().find(|p| p.name == "_outlink"))
    {
        if let Primitive::String(str) = &prop.value {
            let Some(row) = horntail_view_state.get_by_name_paths(str.as_str()) else {
                return;
            };
            if let Some(canvas) = row.to_canvas() {
                *image_offset = offset;
                image_view_state.set_canvas(canvas);
                return;
            }
        }
    }

    *image_offset = offset;
    image_view_state.set_canvas(canvas);
}

fn handle_search(
    horntail_view_state: &mut HorntailViewState,
    text_view_state: &mut InputViewState,
) {
    horntail_view_state.search(
        text_view_state.input_value(),
        text_view_state.prefix() == SEARCH_PREFIX_PREV,
    );
}

fn handle_input_event(
    event: Event,
    event_mode: &mut EventMode,
    horntail_view_state: &mut HorntailViewState,
    text_view_state: &mut InputViewState,
) {
    if let Event::Key(ke) = event {
        match ke.code {
            KeyCode::Char('c') => {
                if ke.modifiers.contains(KeyModifiers::CONTROL) {
                    *event_mode = EventMode::Normal;
                    text_view_state.reset();
                }
            }
            KeyCode::Esc => {
                *event_mode = EventMode::Normal;
                text_view_state.reset();
            }
            KeyCode::Enter => {
                *event_mode = EventMode::Normal;
                handle_search(horntail_view_state, text_view_state);
            }
            _ => {}
        }
    }
    text_view_state.handle_event(&event);
}

fn handle_normal_event(
    ke: KeyEvent,
    last_key_event: Option<&KeyEvent>,
    state: &mut AppState,
    ev_mode: &mut EventMode,
    horntail_view_state: &mut HorntailViewState,
    text_view_state: &mut InputViewState,
) {
    match ke.code {
        KeyCode::Esc => *state = AppState::Stop,
        KeyCode::Char('j') | KeyCode::Down => horntail_view_state.selected_index_next(),
        KeyCode::Char('k') | KeyCode::Up => horntail_view_state.selected_index_prev(),
        KeyCode::Char('e') => {
            if ke.modifiers.contains(KeyModifiers::CONTROL) {
                horntail_view_state.toggle_selected_recursive(None);
            } else {
                horntail_view_state.toggle_selected();
            }
        }
        KeyCode::Char('g') => {
            if let Some(last) = last_key_event {
                if let KeyCode::Char('g') = last.code {
                    horntail_view_state.goto_start();
                }
            }
        }
        KeyCode::Char('G') => {
            if ke.modifiers.contains(KeyModifiers::SHIFT) {
                horntail_view_state.goto_end();
            }
        }
        KeyCode::Char('[') => {
            horntail_view_state.goto_prev_expand_node();
        }
        KeyCode::Char(']') => {
            horntail_view_state.goto_next_expand_node();
        }
        KeyCode::Char('f') => {
            if ke.modifiers.contains(KeyModifiers::CONTROL) {
                horntail_view_state.page_down()
            }
        }
        KeyCode::PageDown => horntail_view_state.page_down(),
        KeyCode::PageUp => horntail_view_state.page_up(),
        KeyCode::Char('b') => {
            if ke.modifiers.contains(KeyModifiers::CONTROL) {
                horntail_view_state.page_up()
            }
        }
        KeyCode::Char('/') => {
            *ev_mode = EventMode::Input;
            text_view_state.reset();
            text_view_state.set_prefix(SEARCH_PREFIX_NEXT)
        }
        KeyCode::Char('?') => {
            *ev_mode = EventMode::Input;
            text_view_state.reset();
            text_view_state.set_prefix(SEARCH_PREFIX_PREV)
        }
        KeyCode::Char('n') => {
            text_view_state.set_prefix(SEARCH_PREFIX_NEXT);
            handle_search(horntail_view_state, text_view_state);
        }
        KeyCode::Char('N') => {
            text_view_state.set_prefix(SEARCH_PREFIX_PREV);
            handle_search(horntail_view_state, text_view_state);
        }
        _ => {}
    };
}

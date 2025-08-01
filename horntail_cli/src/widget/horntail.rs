use crate::error::Error;
use crate::row::{HorntailRow, IndexKind};
use horntail::{EntryKind, ImageKind};
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Constraint, Margin, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget, Table,
    TableState,
};
use std::marker::PhantomData;
use std::path::PathBuf;

const CHILD_MARK: [&str; 2] = ["└─ ", "├─ "];

const SPACE_MARK: [&str; 2] = ["   ", "│  "];

pub struct HorntailViewState<'a> {
    root: HorntailRow,
    block_template: Block<'a>,
    selected_index: usize,
    // calculate properties
    view_offset: usize,
    view_height: usize,
}

impl HorntailViewState<'_> {
    pub fn new<'a>(root: HorntailRow) -> Result<HorntailViewState<'a>, Error> {
        let block_template = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Style::new().light_blue());

        Ok(HorntailViewState {
            root,
            block_template,
            selected_index: 0,
            view_offset: 0,
            view_height: 0,
        })
    }

    #[inline]
    pub fn get_by_name_paths(&mut self, paths: &str) -> Option<&HorntailRow> {
        self.root.get_by_name_paths(paths)
    }

    #[inline]
    pub fn selected(&self) -> &HorntailRow {
        self.root
            .get_by_paths(&get_paths_by_index(&self.root, self.selected_index))
    }

    #[inline]
    pub fn set_selected_index(&mut self, selected_index: usize) {
        self.selected_index = selected_index.clamp(0, self.root.expand_size() - 1)
    }

    #[inline]
    pub fn selected_index_next(&mut self) {
        self.set_selected_index(self.selected_index.saturating_add(1));
    }

    #[inline]
    pub fn selected_index_prev(&mut self) {
        self.set_selected_index(self.selected_index.saturating_sub(1));
    }

    #[inline]
    pub fn page_down(&mut self) {
        self.set_selected_index(self.selected_index.saturating_add(self.view_height));
    }

    #[inline]
    pub fn page_up(&mut self) {
        self.set_selected_index(self.selected_index.saturating_sub(self.view_height));
    }

    #[inline]
    pub fn goto_end(&mut self) {
        self.set_selected_index(self.root.expand_size() - 1);
    }

    #[inline]
    pub fn goto_start(&mut self) {
        self.set_selected_index(0);
    }

    #[inline]
    pub fn goto_next_expand_node(&mut self) {
        let paths = find_next_expand_node(
            &self.root,
            &get_paths_by_index(&self.root, self.selected_index),
        );
        if !paths.is_empty() {
            self.set_selected_index(get_index_by_paths(&self.root, &paths));
        }
    }

    #[inline]
    pub fn goto_prev_expand_node(&mut self) {
        let paths = find_prev_expand_node(
            &self.root,
            &get_paths_by_index(&self.root, self.selected_index),
        );
        if !paths.is_empty() {
            self.set_selected_index(get_index_by_paths(&self.root, &paths));
        }
    }

    #[inline]
    pub fn toggle_selected(&mut self) {
        self.root
            .toggle_paths(&get_paths_by_index(&self.root, self.selected_index));
    }

    #[inline]
    pub fn toggle_selected_recursive(&mut self, depth: Option<usize>) {
        self.root
            .toggle_recursive(&get_paths_by_index(&self.root, self.selected_index), depth);
    }

    pub fn search(&mut self, text: &str, prev: bool) {
        let paths = if prev {
            find_prev_node(
                &self.root,
                &get_paths_by_index(&self.root, self.selected_index),
                text,
            )
        } else {
            find_next_node(
                &self.root,
                &get_paths_by_index(&self.root, self.selected_index),
                text,
            )
        };

        if !paths.is_empty() {
            self.root.expand_paths(&paths[..paths.len() - 1]);
            self.set_selected_index(get_index_by_paths(&self.root, &paths));
        }
    }
}

pub struct HorntailView<'a> {
    _phantom: &'a PhantomData<()>,
}

impl HorntailView<'_> {
    pub fn new<'a>() -> HorntailView<'a> {
        HorntailView {
            _phantom: &PhantomData,
        }
    }

    fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut HorntailViewState) {
        let table_area = area.inner(Margin::new(0, 1));

        state.view_offset = if state.selected_index
            >= state.view_offset.saturating_add(table_area.height as usize)
        {
            state
                .selected_index
                .saturating_sub(table_area.height.saturating_sub(1) as usize)
        } else if state.selected_index < state.view_offset {
            state.selected_index
        } else if state.root.expand_size().saturating_sub(state.view_offset)
            < table_area.height as usize
        {
            state
                .root
                .expand_size()
                .saturating_sub(table_area.height as usize)
        } else {
            state.view_offset
        };

        state.view_height = table_area.height as usize;

        let mut scrollbar_state =
            ScrollbarState::new(state.root.expand_size()).position(state.selected_index);
        let mut table_state = TableState::new()
            .with_selected(Some(state.selected_index.saturating_sub(state.view_offset)));

        Table::new(
            build_table_rows(&state.root, state.view_offset, table_area.height as usize),
            [
                Constraint::Length(50),
                Constraint::Length(10),
                Constraint::Length(10),
                Constraint::Length(18),
                Constraint::Fill(0),
            ],
        )
        .row_highlight_style(Style::new().black().on_white())
        .block(state.block_template.clone().title("Resource"))
        .render(area, buf, &mut table_state);

        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .track_symbol(None)
            .render(table_area, buf, &mut scrollbar_state);
    }
}

impl<'a> StatefulWidget for HorntailView<'a> {
    type State = HorntailViewState<'a>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        self.render_ref(area, buf, state)
    }
}

fn colorful_data_type(row: &HorntailRow) -> Span {
    let style = match row.kind {
        IndexKind::Primitive(_) => Style::new().light_green(),
        IndexKind::Element(kind) => match kind {
            EntryKind::Folder | EntryKind::Property(_) => Style::new().light_cyan(),
            EntryKind::Image(ik) => match ik {
                ImageKind::Canvas | ImageKind::Video | ImageKind::Sound => {
                    Style::new().light_yellow()
                }
                ImageKind::UOL => Style::new().light_magenta(),
                ImageKind::RawData => Style::new().light_cyan(),
                ImageKind::Vector2D => Style::new().light_green(),
                _ => Style::default(),
            },
        },
    };
    Span::styled(row.name(), style)
}

fn get_paths_by_index(cache: &HorntailRow, index: usize) -> Vec<usize> {
    fn _find_paths(cache: &HorntailRow, index: usize, mut acc: usize) -> Vec<usize> {
        let mut result = Vec::with_capacity(0);
        for (pos, leaf) in cache.children().iter().enumerate() {
            let temp = acc + leaf.expand_size();
            if temp < index {
                acc = temp + 1;
                continue;
            }
            result.insert(0, pos);
            if acc < index {
                result.extend(_find_paths(leaf, index, acc + 1));
            }
            return result;
        }
        result
    }

    _find_paths(cache, index, 0)
}

fn get_index_by_paths(root: &HorntailRow, paths: &[usize]) -> usize {
    paths
        .iter()
        .copied()
        .fold((root, 0), |(cursor, mut acc), index| {
            if cursor.is_expand() {
                acc += cursor.children()[..index]
                    .iter()
                    .fold(0, |acc, leaf| acc + leaf.expand_size())
                    + 1;
            }
            (&cursor.children()[index], acc + index)
        })
        .1
        - 1
}

fn build_table_rows(root: &HorntailRow, view_offset: usize, mut view_size: usize) -> Vec<Row> {
    fn _build_row(prefix: String, row: &HorntailRow) -> Row {
        Row::new([
            Line::from(vec![
                Span::styled(prefix, Style::new().light_blue()),
                colorful_data_type(row),
            ]),
            Line::from(format!("{:#X}", row.offset)).alignment(Alignment::Left),
            Line::from(row.kind.as_str()),
            Line::from(
                PathBuf::from(&*row.group.file)
                    .file_name()
                    .and_then(|x| x.to_str().map(|s| s.to_owned()))
                    .unwrap_or("".to_string()),
            ),
            Line::from(row.value()),
        ])
    }

    fn _build_rows<'a>(
        prefix: String,
        cache: &'a [HorntailRow],
        rows: &mut Vec<Row<'a>>,
        view_size: &mut usize,
    ) {
        let mut cache_iter = cache.iter().enumerate();
        let size = cache.len().saturating_sub(1);

        let child_mask = (
            prefix.clone() + CHILD_MARK[0],
            prefix.clone() + CHILD_MARK[1],
        );
        let space_mask = (
            prefix.clone() + SPACE_MARK[0],
            prefix.clone() + SPACE_MARK[1],
        );

        while let (true, Some((index, row))) = (*view_size > 0, cache_iter.next()) {
            rows.push(_build_row(
                if index == size {
                    child_mask.0.clone()
                } else {
                    child_mask.1.clone()
                },
                row,
            ));
            *view_size -= 1;
            if row.is_expand() {
                _build_rows(
                    if index == size {
                        space_mask.0.clone()
                    } else {
                        space_mask.1.clone()
                    },
                    row.children(),
                    rows,
                    view_size,
                );
            }
        }
    }

    fn _build_view<'a>(
        prefix: String,
        cache: &'a [HorntailRow],
        paths: &[usize],
        rows: &mut Vec<Row<'a>>,
        view_size: &mut usize,
    ) {
        let Some((first, last)) = paths.split_first() else {
            return;
        };
        let cache = if !last.is_empty() {
            let mask = if *first == cache.len().saturating_sub(1) {
                SPACE_MARK[0]
            } else {
                SPACE_MARK[1]
            };
            _build_view(
                prefix.clone() + mask,
                cache[*first].children(),
                last,
                rows,
                view_size,
            );
            &cache[*first + 1..]
        } else {
            &cache[*first..]
        };

        if !cache.is_empty() && *view_size > 0 {
            _build_rows(prefix, cache, rows, view_size);
        }
    }

    let mut rows = Vec::with_capacity(0);
    _build_view(
        String::new(),
        root.children(),
        &get_paths_by_index(root, view_offset),
        &mut rows,
        &mut view_size,
    );
    rows
}

fn find_next_expand_node(cache: &HorntailRow, paths: &[usize]) -> Vec<usize> {
    fn _find_next_expand(cache: &HorntailRow, paths: &[usize]) -> Vec<usize> {
        let mut skip = 0;
        if let Some((first, last)) = paths.split_first() {
            let mut result = _find_next_expand(&cache.children()[*first], last);
            if !result.is_empty() {
                result.insert(0, *first);
                return result;
            }
            skip = first.saturating_add(1)
        };

        if !cache.is_expand() {
            return Vec::with_capacity(0);
        }

        cache
            .children()
            .iter()
            .enumerate()
            .skip(skip)
            .find(|(_, leaf)| leaf.is_expand())
            .map(|(index, _)| vec![index])
            .unwrap_or_default()
    }

    _find_next_expand(cache, paths)
}

fn find_next_node(cache: &HorntailRow, paths: &[usize], text: &str) -> Vec<usize> {
    if text.is_empty() {
        return Vec::with_capacity(0);
    }

    fn _find_next_node(cache: &HorntailRow, paths: &[usize], text: &str) -> Vec<usize> {
        let mut skip = 0;
        if let Some((first, last)) = paths.split_first() {
            let mut result = _find_next_node(&cache.children()[*first], last, text);
            if !result.is_empty() {
                result.insert(0, *first);
                return result;
            }
            skip = *first + 1;
        }

        cache
            .children()
            .iter()
            .enumerate()
            .skip(skip)
            .find_map(|(index, leaf)| {
                if leaf.name().contains(text) {
                    return Some(vec![index]);
                }
                let mut result = _find_next_node(leaf, &[], text);
                if !result.is_empty() {
                    result.insert(0, index);
                    return Some(result);
                }
                None
            })
            .unwrap_or_default()
    }

    _find_next_node(cache, paths, text)
}

fn find_prev_expand_node(cache: &HorntailRow, paths: &[usize]) -> Vec<usize> {
    fn _rfind_expand_node(cache: &[HorntailRow]) -> Vec<usize> {
        let mut result = Vec::with_capacity(0);
        cache.iter().enumerate().rfind(|(index, leaf)| {
            if !leaf.is_expand() {
                return false;
            }
            result.push(*index);
            result.extend(_rfind_expand_node(leaf.children()));
            true
        });
        result
    }

    fn _find_prev_expand(cache: &HorntailRow, paths: &[usize]) -> Vec<usize> {
        let Some((first, last)) = paths.split_first() else {
            return Vec::with_capacity(0);
        };
        if !last.is_empty() {
            let mut result = _find_prev_expand(&cache.children()[*first], last);
            result.insert(0, *first);
            return result;
        }
        _rfind_expand_node(&cache.children()[..*first])
    }

    _find_prev_expand(cache, paths)
}

fn find_prev_node(cache: &HorntailRow, paths: &[usize], text: &str) -> Vec<usize> {
    if text.is_empty() {
        return Vec::with_capacity(0);
    }

    fn _rfind_node(cache: &[HorntailRow], text: &str) -> Vec<usize> {
        let mut result = Vec::with_capacity(0);
        for (index, leaf) in cache.iter().enumerate().rev() {
            let leaf_result = _rfind_node(leaf.children(), text);
            if !leaf_result.is_empty() {
                result.insert(0, index);
                result.extend(leaf_result);
                break;
            } else if leaf.name().contains(text) {
                result.push(index);
                break;
            }
        }
        result
    }

    fn _find_prev_node(cache: &HorntailRow, paths: &[usize], text: &str) -> Vec<usize> {
        let Some((first, last)) = paths.split_first() else {
            return Vec::with_capacity(0);
        };
        if !last.is_empty() {
            let leaf = &cache.children()[*first];
            let mut result = _find_prev_node(leaf, last, text);
            if leaf.name().contains(text) || !result.is_empty() {
                result.insert(0, *first);
                return result;
            }
        }
        _rfind_node(&cache.children()[..*first], text)
    }

    _find_prev_node(cache, paths, text)
}

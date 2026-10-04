//! Rendering, style composition, Unicode clipping and shared hit-test geometry.
use super::*;
use ratatui_core::{buffer::Buffer, widgets::Widget};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

impl<T> Grid<T> {
    pub(super) fn render(&mut self, area: Rect, buffer: &mut Buffer) {
        let area = area.intersection(buffer.area);
        let resized = self.area.width != area.width || self.area.height != area.height;
        if self.area != area {
            self.hover = None;
            self.clear_cell_details();
            self.drag = None;
        }
        self.area = area;
        self.clamp_page();
        let range = self.row_range();
        self.row_offset = self
            .row_offset
            .min(range.len().saturating_sub(self.body_height()));
        self.column_offset = self.column_offset.min(self.max_column_offset());
        if resized {
            self.reveal_selected_row();
            if let Some((_, column)) = self.cursor() {
                self.reveal_column(column);
            }
        }
        self.layout.clear();
        self.retry_button = None;
        let pinned_width = self.pinned_width().min(usize::from(area.width));
        let boundary = area.x + pinned_width as u16;
        let mut pinned_start = i64::from(area.x);
        let mut scrolling_start = i64::from(boundary) - self.column_offset as i64;
        for column in self.visible_columns() {
            let pinned = self.column_pinned[column];
            let start = if pinned {
                pinned_start
            } else {
                scrolling_start
            };
            let end = start + i64::from(self.model.columns[column].width);
            if pinned {
                pinned_start = end;
            } else {
                scrolling_start = end;
            }
            let left = if pinned { area.x } else { boundary };
            let right = if pinned { boundary } else { area.right() };
            let clip_start = start.clamp(i64::from(left), i64::from(right)) as u16;
            let clip_end = end.clamp(i64::from(left), i64::from(right)) as u16;
            self.layout.push(ColumnLayout {
                column,
                start,
                end,
                clip_start,
                clip_end,
            });
        }
        if area.is_empty() {
            return;
        }
        let selected_position = self.model.selected_position();
        let cursor = self.cursor();
        for y in area.y..area.bottom().saturating_sub(u16::from(self.has_footer())) {
            let position = range.start
                + self.row_offset
                + usize::from(y.saturating_sub(area.y).saturating_sub(1));
            let base = if y != area.y
                && self
                    .model
                    .index_at(position)
                    .is_some_and(|index| self.model.is_row_selected(index))
                && range.contains(&position)
            {
                self.style.cell.patch(self.style.marked)
            } else {
                self.style.cell
            };
            let style = if y == area.y {
                self.style.header
            } else if selected_position == Some(position) && range.contains(&position) {
                base.patch(self.style.selected)
            } else if self.hover == Some(Hit::Row(position)) {
                base.patch(self.style.hover)
            } else {
                base
            };
            for x in area.x..area.right() {
                buffer[(x, y)].reset();
                buffer[(x, y)].set_style(style);
            }
            for layout in &self.layout {
                if layout.clip_start >= layout.clip_end {
                    continue;
                }
                let definition = &self.model.columns[layout.column];
                let (text, style) = if y == area.y {
                    let suffix = match self.model.sort() {
                        Some(Sort { column, direction }) if column == layout.column => {
                            match direction {
                                SortDirection::Ascending => " ↑",
                                SortDirection::Descending => " ↓",
                            }
                        }
                        _ if definition.compare.is_some()
                            || (self.is_external() && definition.external_sortable) =>
                        {
                            " ↕"
                        }
                        _ => "",
                    };
                    let style = if self.header_focus == Some(layout.column) {
                        self.style.header.patch(self.style.focused_header)
                    } else if matches!(self.hover, Some(Hit::Header(c) | Hit::Resize(c)) if c == layout.column)
                    {
                        self.style.header.patch(self.style.hover)
                    } else {
                        style
                    };
                    for x in layout.clip_start..layout.clip_end {
                        buffer[(x, y)].set_style(style);
                    }
                    let space = usize::from(definition.width.saturating_sub(1))
                        .saturating_sub(UnicodeWidthStr::width(suffix));
                    let title = fit_text(&definition.title, space);
                    (format!("{title}{suffix}"), style)
                } else if let Some(row) =
                    self.model.row_at(position).filter(|_| position < range.end)
                {
                    let index = self.model.index_at(position).expect("resident row");
                    let mut style = self.style.cell;
                    if let Some(cell_style) = &definition.cell_style {
                        style = style.patch(cell_style(row));
                    }
                    if self.model.is_row_selected(index) {
                        style = style.patch(self.style.marked);
                    }
                    if selected_position == Some(position) {
                        style = style.patch(self.style.selected);
                    } else if self.hover == Some(Hit::Row(position)) {
                        style = style.patch(self.style.hover);
                    }
                    let style = self
                        .flashes
                        .get(&(index, layout.column))
                        .map_or(style, |flash| {
                            let remaining =
                                1.0 - flash.elapsed.as_secs_f64() / flash.duration.as_secs_f64();
                            faded_style(style, self.style.flash, remaining)
                        });
                    let style = if cursor == Some((index, layout.column)) {
                        style.patch(self.style.cursor)
                    } else {
                        style
                    };
                    for x in layout.clip_start..layout.clip_end {
                        buffer[(x, y)].set_style(style);
                    }
                    (
                        fit_text(&(definition.format)(row), usize::from(definition.width - 1)),
                        style,
                    )
                } else {
                    (String::new(), style)
                };
                let cell_area =
                    Rect::new(layout.clip_start, y, layout.clip_end - layout.clip_start, 1);
                write_clipped(
                    buffer,
                    cell_area,
                    y,
                    layout.start,
                    layout.end - 1,
                    &text,
                    style,
                );
                let separator = layout.end - 1;
                if separator >= i64::from(layout.clip_start)
                    && separator < i64::from(layout.clip_end)
                {
                    let dragging = self.drag.is_some_and(|drag| drag.column == layout.column);
                    let resize_handle = y == area.y
                        && (dragging
                            || (self.drag.is_none()
                                && self.hover == Some(Hit::Resize(layout.column))));
                    let separator_style = style.patch(self.style.separator);
                    let separator_style = if resize_handle {
                        separator_style.patch(if dragging {
                            self.style.focused_header
                        } else {
                            self.style.hover
                        })
                    } else {
                        separator_style
                    };
                    buffer[(separator as u16, y)]
                        .set_symbol(if resize_handle { "↔" } else { "│" })
                        .set_style(separator_style);
                }
            }
        }
        self.render_status(buffer);
        self.render_pager(buffer);
        self.render_cell_details(buffer);
    }

    fn render_status(&mut self, buffer: &mut Buffer) {
        if self.body_height() == 0 {
            return;
        }
        let y = self.area.y + 1;
        let (message, style, retry) = match self.load_state() {
            LoadState::Loading => (
                "Loading records…".to_owned(),
                self.style.cell.patch(self.style.status),
                false,
            ),
            LoadState::Error(message) => (
                format!("[Retry] {message} · F5 to retry"),
                self.style.cell.patch(self.style.error),
                true,
            ),
            LoadState::Empty => (
                if !self.search_query.is_empty() || !self.model.rows().is_empty() {
                    "No matching records"
                } else {
                    "No records"
                }
                .into(),
                self.style.cell.patch(self.style.status),
                false,
            ),
            LoadState::Ready => return,
        };
        for x in self.area.x..self.area.right() {
            buffer[(x, y)].reset();
            buffer[(x, y)].set_style(style);
        }
        write_clipped(
            buffer,
            self.area,
            y,
            i64::from(self.area.x),
            i64::from(self.area.right()),
            &message,
            style,
        );
        if retry {
            self.retry_button = Some(Rect::new(self.area.x, y, self.area.width.min(7), 1));
        }
    }

    fn render_pager(&mut self, buffer: &mut Buffer) {
        self.page_buttons.clear();
        if !self.has_footer() {
            return;
        }
        let y = self.area.bottom() - 1;
        if self.is_searching() || self.pagination.is_none() {
            let text = self.search_draft.as_ref().map_or_else(
                || {
                    format!(
                        "Search: {} · {} matching · / to search",
                        self.search_query,
                        self.model.visible_len()
                    )
                },
                |draft| format!("/ {draft}▏ · Enter applies · Esc cancels"),
            );
            let style = self.style.header.patch(self.style.status);
            for x in self.area.x..self.area.right() {
                buffer[(x, y)].reset();
                buffer[(x, y)].set_style(style);
            }
            write_clipped(
                buffer,
                self.area,
                y,
                i64::from(self.area.x),
                i64::from(self.area.right()),
                &text,
                style,
            );
            return;
        }
        let state = self.page_state().expect("pagination footer");
        let mut x = self.area.x;
        for cell_x in self.area.x..self.area.right() {
            buffer[(cell_x, y)].reset();
            buffer[(cell_x, y)].set_style(self.style.header);
        }
        for (label, navigation, enabled) in [
            ("[<<]", Navigation::First, state.page > 0),
            ("[<]", Navigation::Previous, state.page > 0),
            ("[>]", Navigation::Next, state.has_next_page),
            (
                "[>>]",
                Navigation::Last,
                state.page_count.is_some_and(|count| state.page < count - 1),
            ),
        ] {
            if x >= self.area.right() {
                break;
            }
            let width = (label.len() as u16).min(self.area.right() - x);
            let rect = Rect::new(x, y, width, 1);
            let style = if !enabled {
                self.style.header.patch(self.style.separator)
            } else if self.hover == Some(Hit::Page(navigation)) {
                self.style.header.patch(self.style.hover)
            } else {
                self.style.header
            };
            for cell_x in rect.x..rect.right() {
                buffer[(cell_x, y)].set_style(style);
            }
            write_clipped(
                buffer,
                self.area,
                y,
                i64::from(x),
                i64::from(x + width),
                label,
                style,
            );
            self.page_buttons.push((rect, navigation, enabled));
            x = x.saturating_add(width).saturating_add(1);
        }
        let summary = if state.mode == PaginationMode::Cursor {
            format!(
                "Batch {} · {} records",
                state.page.saturating_add(1),
                state.loaded_rows,
            )
        } else {
            let offset = state.page.saturating_mul(state.page_size.get());
            let (range_rows, total, count) = match (state.total_rows, state.page_count) {
                (Some(total), Some(count)) => (
                    total.saturating_sub(offset).min(state.page_size.get()),
                    total.to_string(),
                    format!("/{count}"),
                ),
                _ => (state.loaded_rows, "?".into(), String::new()),
            };
            let start = if range_rows == 0 {
                0
            } else {
                offset.saturating_add(1)
            };
            let end = if range_rows == 0 {
                0
            } else {
                offset.saturating_add(range_rows)
            };
            format!(
                "Page {}{} · {}–{} / {}",
                state.page.saturating_add(1),
                count,
                start,
                end,
                total,
            )
        };
        let text = format!(
            "{summary}{}{}",
            if state.loading { " · loading" } else { "" },
            if self.search_query.is_empty() {
                String::new()
            } else {
                format!(" · Search: {}", self.search_query)
            }
        );
        write_clipped(
            buffer,
            self.area,
            y,
            i64::from(x),
            i64::from(self.area.right()),
            &text,
            self.style.header,
        );
    }
}

fn faded_style(base: Style, flash: Style, remaining: f64) -> Style {
    let mut result = if remaining > 0.5 {
        base.patch(flash)
    } else {
        base
    };
    fn blend(base: Option<Color>, flash: Option<Color>, remaining: f64) -> Option<Color> {
        match (base, flash) {
            (Some(Color::Rgb(r, g, b)), Some(Color::Rgb(fr, fg, fb))) => {
                let channel = |a: u8, f: u8| {
                    (f64::from(a) + (f64::from(f) - f64::from(a)) * remaining).round() as u8
                };
                Some(Color::Rgb(channel(r, fr), channel(g, fg), channel(b, fb)))
            }
            _ if remaining > 0.5 => flash.or(base),
            _ => base,
        }
    }
    result.fg = blend(base.fg, flash.fg, remaining);
    result.bg = blend(base.bg, flash.bg, remaining);
    result
}

impl<T> Widget for GridWidget<'_, T> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        self.0.render(area, buffer);
    }
}

// Fit by terminal cells, retaining complete graphemes and indicating overflow.
// Sanitization precedes segmentation; clipping stops at the first overflow.
// Inspect geometrically growing chunks until a complete grapheme overflows.
// Keep the final grapheme pending: later text (including text joined across
// removed controls) can extend it. Each source byte is sanitized at most once.
fn sanitized_prefix(text: &str, cells: usize) -> std::borrow::Cow<'_, str> {
    use std::borrow::Cow;
    if text.len() <= 128 {
        return crate::text::sanitize(text);
    }
    let mut prefix = Cow::Borrowed("");
    let mut start = 0;
    let mut end = text.floor_char_boundary(128);
    loop {
        let chunk = crate::text::sanitize(&text[start..end]);
        if matches!(prefix, Cow::Borrowed(_)) && matches!(chunk, Cow::Borrowed(_)) {
            prefix = Cow::Borrowed(&text[..end]);
        } else {
            prefix.to_mut().push_str(&chunk);
        }
        if end == text.len() {
            return prefix;
        }
        let mut graphemes = prefix.graphemes(true).peekable();
        let mut used = 0;
        while let Some(grapheme) = graphemes.next() {
            if graphemes.peek().is_none() {
                break;
            }
            let width = UnicodeWidthStr::width(grapheme);
            if used + width > cells {
                return prefix;
            }
            used += width;
        }
        start = end;
        end = text.floor_char_boundary(end.saturating_mul(2).min(text.len()));
    }
}

fn fit_text(text: &str, cells: usize) -> String {
    if cells == 0 {
        return String::new();
    }
    let mut fitted = String::new();
    let mut used = 0;
    let text = sanitized_prefix(text, cells);
    for grapheme in text.graphemes(true) {
        let width = UnicodeWidthStr::width(grapheme);
        if width == 0 {
            continue;
        }
        if used + width > cells {
            while used > cells - 1 {
                let last = fitted.graphemes(true).next_back().expect("fitted grapheme");
                used -= UnicodeWidthStr::width(last);
                fitted.truncate(fitted.len() - last.len());
            }
            // A removed wide grapheme can leave one spare cell before the marker.
            fitted.extend(std::iter::repeat_n(' ', cells - 1 - used));
            fitted.push('…');
            return fitted;
        }
        fitted.push_str(grapheme);
        used += width;
    }
    fitted
}

fn write_clipped(
    buffer: &mut Buffer,
    area: Rect,
    y: u16,
    start: i64,
    end: i64,
    text: &str,
    style: Style,
) {
    let mut x = start;
    let end = end.min(i64::from(area.right()));
    let text = sanitized_prefix(text, end.saturating_sub(start).max(0) as usize);
    for grapheme in text.graphemes(true) {
        let width = UnicodeWidthStr::width(grapheme) as i64;
        if width == 0 {
            continue;
        }
        if x + width > end {
            break;
        }
        if x >= i64::from(area.x) && x + width <= i64::from(area.right()) {
            buffer[(x as u16, y)].set_symbol(grapheme).set_style(style);
            for continuation in 1..width {
                buffer[((x + continuation) as u16, y)]
                    .set_symbol(" ")
                    .set_style(style);
            }
        }
        x += width;
    }
}

#[cfg(test)]
mod long_text_tests {
    use super::*;

    // Whole-text oracle: choose the longest prefix that leaves one cell for an
    // ellipsis, instead of using the renderer's overflow/backtracking algorithm.
    fn reference_fit(text: &str, cells: usize) -> String {
        if cells == 0 {
            return String::new();
        }
        let text = crate::text::sanitize(text);
        let graphemes: Vec<_> = text
            .graphemes(true)
            .map(|g| (g, UnicodeWidthStr::width(g)))
            .filter(|(_, width)| *width > 0)
            .collect();
        let total: usize = graphemes.iter().map(|(_, width)| width).sum();
        if total <= cells {
            return graphemes.iter().map(|(g, _)| *g).collect();
        }
        let mut result = String::new();
        let mut used = 0;
        for (g, width) in graphemes {
            if used + width > cells - 1 {
                break;
            }
            result.push_str(g);
            used += width;
        }
        result.extend(std::iter::repeat_n(' ', cells - 1 - used));
        result.push('…');
        result
    }

    #[test]
    fn long_visible_prefix_does_not_scan_or_allocate_the_hidden_tail() {
        let text = format!("{}\u{202e}hidden", "a".repeat(1_048_576));
        let prefix = sanitized_prefix(&text, 20);
        assert!(matches!(prefix, std::borrow::Cow::Borrowed(_)));
        assert_eq!(prefix.len(), 128);
        let text = format!("\u{202e}{}\x1bhidden", "a".repeat(1_048_576));
        assert!(sanitized_prefix(&text, 20).len() <= 128);
        assert_eq!(fit_text(&text, 20), format!("{}…", "a".repeat(19)));
    }

    #[test]
    fn chunk_boundaries_preserve_graphemes_joined_across_removed_controls() {
        for prefix_len in [0, 123, 124, 125, 126, 127, 128, 129, 250, 255, 256, 257] {
            for joined in [
                "e\u{202e}\u{301}",
                "👩\u{2066}\u{200d}💻",
                "👩\x1b🏽\u{200d}💻",
                "🇺\u{061c}🇸",
                "\u{0600}\u{202e}a",
                "क्\u{202e}ष",
                "\r\n",
                "e\u{301}\u{301}\u{301}",
                "\u{200d}\u{fe0f}",
            ] {
                let input = format!(
                    "{}{joined}{}",
                    "a".repeat(prefix_len),
                    "tail 東京 ".repeat(50)
                );
                for cells in [0, 1, 2, 19, 20, 64, 127, 128, 129, 255, 256, 257, 1024] {
                    assert_eq!(
                        fit_text(&input, cells),
                        reference_fit(&input, cells),
                        "prefix={prefix_len}, cells={cells}, joined={joined:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn control_runs_and_unbounded_combining_clusters_remain_correct() {
        for input in [
            format!("{}e\u{202e}\u{301}tail", "\u{2066}\x1b\n".repeat(20_000)),
            format!("e{}tail", "\u{301}".repeat(20_000)),
            "\u{202e}\u{2069}\x1b".repeat(20_000),
            "🇺".repeat(1000),
        ] {
            for cells in [0, 1, 2, 20, 128, 256] {
                assert_eq!(fit_text(&input, cells), reference_fit(&input, cells));
            }
        }
    }

    #[test]
    fn seeded_hostile_strings_match_full_sanitization_and_clipping() {
        let pieces = [
            "a",
            "東",
            "e\u{301}",
            "👩🏽‍💻",
            "🇺",
            "🇸",
            "\u{200d}",
            "\u{fe0f}",
            "\u{202e}",
            "\u{2066}",
            "\u{2069}",
            "\x1b",
            "\t",
            "\n",
            "\u{061c}",
            "\u{0600}",
            "\u{301}",
            "क्",
            "ष",
        ];
        let mut seed = 0x99ab42u64;
        for step in 0..512 {
            let mut input = String::new();
            for _ in 0..(step % 256 + 64) {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                input.push_str(pieces[(seed >> 32) as usize % pieces.len()]);
            }
            for cells in [1, 2, 3, 20, 64, 128, 256] {
                assert_eq!(
                    fit_text(&input, cells),
                    reference_fit(&input, cells),
                    "step={step}, cells={cells}"
                );
            }
        }
    }
}

#[cfg(test)]
mod long_clipped_tests {
    use super::*;

    #[test]
    fn horizontal_clipping_skips_partial_graphemes_and_hidden_long_tail() {
        let text = format!("東👩🏽‍💻e\u{202e}\u{301}abc{}", "Z".repeat(1_048_576));
        let area = Rect::new(3, 4, 5, 1);
        let mut buffer = Buffer::empty(area);
        write_clipped(&mut buffer, area, 4, 0, 1_048_576, &text, Style::default());
        let symbols: Vec<_> = buffer.content().iter().map(|c| c.symbol()).collect();
        assert_eq!(symbols, [" ", "e\u{301}", "a", "b", "c"]);
    }
}

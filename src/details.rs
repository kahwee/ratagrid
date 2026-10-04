//! Optional full-value inspection, with application-driven hover timing.
use super::*;
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers, MouseEventKind};
use ratatui_core::buffer::Buffer;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// Opt-in full-value panels for truncated data cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellDetailsOptions {
    /// How long a pointer must stay on one cell before its panel opens.
    pub hover_delay: Duration,
    /// Maximum panel height, including its title and border (minimum four).
    pub max_height: u16,
}
impl Default for CellDetailsOptions {
    fn default() -> Self {
        Self {
            hover_delay: Duration::from_millis(500),
            max_height: 8,
        }
    }
}

/// The complete, printable value currently shown in a detail panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellDetail {
    /// Insertion index into the current resident records.
    pub row: usize,
    pub column: usize,
    pub title: String,
    pub value: String,
}
struct HoverDetail {
    cell: (usize, usize),
    point: (u16, u16),
    elapsed: Duration,
}
struct OpenDetail {
    content: CellDetail,
    keyboard: bool,
    scroll: usize,
}
#[derive(Default)]
pub(super) struct DetailsState {
    options: Option<CellDetailsOptions>,
    hover: Option<HoverDetail>,
    open: Option<OpenDetail>,
    panel: Rect,
}

impl<T> Grid<T> {
    /// Enable full-value inspection. Disabled by default.
    /// Call [`Self::advance_cell_details`] with elapsed frame time for hover opening.
    /// Enter opens a truncated cursor cell; Escape closes it without clearing selection.
    /// Up/Down and PageUp/PageDown scroll a keyboard-opened panel.
    pub fn with_cell_details(mut self, options: CellDetailsOptions) -> Self {
        self.set_cell_details(Some(options));
        self
    }
    /// Enable, configure, or disable inspection, closing any existing panel.
    pub fn set_cell_details(&mut self, options: Option<CellDetailsOptions>) {
        self.clear_cell_details();
        self.details.options = options;
    }
    /// Read the active configuration; `None` means inspection is disabled.
    pub fn cell_details_options(&self) -> Option<CellDetailsOptions> {
        self.details.options
    }
    /// Read the full value snapshot currently displayed, if any.
    pub fn cell_detail(&self) -> Option<&CellDetail> {
        self.details.open.as_ref().map(|open| &open.content)
    }
    /// Whether a keyboard-opened panel owns navigation keys.
    pub fn is_inspecting_cell(&self) -> bool {
        self.details.open.as_ref().is_some_and(|open| open.keyboard)
    }
    /// Advance the hover delay. Returns true when a panel opens (redraw required).
    /// Uses no wall clock; applications should call this even when animations are off.
    pub fn advance_cell_details(&mut self, elapsed: Duration) -> bool {
        let Some(options) = self.details.options else {
            return false;
        };
        let Some(hover) = self.details.hover.as_mut() else {
            return false;
        };
        hover.elapsed = hover.elapsed.saturating_add(elapsed);
        if hover.elapsed < options.hover_delay || self.details.open.is_some() {
            return false;
        }
        let cell = hover.cell;
        let point = hover.point;
        if self.detail_hit(point) != Some(cell) {
            self.clear_cell_details();
            return false;
        }
        let Some(content) = self.detail_content(cell) else {
            self.clear_cell_details();
            return false;
        };
        self.details.open = Some(OpenDetail {
            content,
            keyboard: false,
            scroll: 0,
        });
        true
    }
    pub(super) fn clear_cell_details(&mut self) {
        self.details.hover = None;
        self.details.open = None;
        self.details.panel = Rect::default();
    }
    fn detail_hit(&self, point: (u16, u16)) -> Option<(usize, usize)> {
        let Hit::Row(position) = self.hit(point.0, point.1)? else {
            return None;
        };
        let layout = self
            .layout
            .iter()
            .find(|c| point.0 >= c.clip_start && point.0 < c.clip_end)?;
        // Separators are not cell content.
        if i64::from(point.0) == layout.end - 1 {
            return None;
        }
        Some((self.model.index_at(position)?, layout.column))
    }
    fn detail_content(&self, (row, column): (usize, usize)) -> Option<CellDetail> {
        if self.area.width < 8
            || self.area.height < 4
            || self.is_searching()
            || !matches!(self.load_state(), LoadState::Ready)
            || !self.column_visible[column]
        {
            return None;
        }
        let record = self.model.rows().get(row)?;
        let definition = self.model.columns.get(column)?;
        let value = printable(&(definition.format)(record));
        if UnicodeWidthStr::width(value.as_str()) <= usize::from(definition.width - 1) {
            return None;
        }
        Some(CellDetail {
            row,
            column,
            title: printable(&definition.title),
            value,
        })
    }
    /// Route events through the optional inspector and then normal grid interaction.
    /// Render again before handling another mouse event.
    pub fn handle_event(&mut self, event: &Event) -> Option<Action> {
        if self.details.options.is_none() {
            return self.handle_grid_event(event);
        }
        if let Event::Key(key) = event {
            if key.kind == KeyEventKind::Release {
                return self.handle_grid_event(event);
            }
            let plain = !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER);
            if plain && !self.is_searching() {
                if key.code == KeyCode::Esc && self.details.open.is_some() {
                    self.clear_cell_details();
                    return Some(Action::CellDetailsChanged);
                }
                if self.is_inspecting_cell()
                    && matches!(
                        key.code,
                        KeyCode::Up
                            | KeyCode::Down
                            | KeyCode::PageUp
                            | KeyCode::PageDown
                            | KeyCode::Home
                            | KeyCode::End
                    )
                {
                    self.scroll_detail(key.code);
                    return Some(Action::CellDetailsChanged);
                }
                if key.code == KeyCode::Enter && self.header_focus.is_none() {
                    let cell = self
                        .cursor()
                        .or_else(|| self.details.hover.as_ref().map(|hover| hover.cell));
                    if let Some(content) = cell.and_then(|cell| self.detail_content(cell)) {
                        self.details.open = Some(OpenDetail {
                            content,
                            keyboard: true,
                            scroll: 0,
                        });
                        self.details.hover = None;
                        return Some(Action::CellDetailsChanged);
                    }
                }
            }
        }
        if let Event::Mouse(mouse) = event {
            if self.details.open.is_some()
                && self
                    .details
                    .panel
                    .contains((mouse.column, mouse.row).into())
            {
                match mouse.kind {
                    MouseEventKind::Moved => return None,
                    MouseEventKind::Down(_) => {
                        self.clear_cell_details();
                        return Some(Action::CellDetailsChanged);
                    }
                    MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
                        self.scroll_detail(if mouse.kind == MouseEventKind::ScrollUp {
                            KeyCode::Up
                        } else {
                            KeyCode::Down
                        });
                        return Some(Action::CellDetailsChanged);
                    }
                    _ => (),
                }
            }
            if mouse.kind == MouseEventKind::Moved && self.drag.is_none() && !self.is_searching() {
                let cell = self.detail_hit((mouse.column, mouse.row));
                if cell == self.details.hover.as_ref().map(|hover| hover.cell) {
                    if let Some(hover) = self.details.hover.as_mut() {
                        hover.point = (mouse.column, mouse.row);
                    }
                    return self.handle_grid_event(event);
                }
                let was_open = self.details.open.is_some();
                self.clear_cell_details();
                if let Some(cell) = cell {
                    self.details.hover = Some(HoverDetail {
                        cell,
                        point: (mouse.column, mouse.row),
                        elapsed: Duration::ZERO,
                    });
                }
                let action = self.handle_grid_event(event);
                return action.or(was_open.then_some(Action::CellDetailsChanged));
            }
        }
        let was_open = self.details.open.is_some();
        self.clear_cell_details();
        self.handle_grid_event(event)
            .or(was_open.then_some(Action::CellDetailsChanged))
    }
    fn scroll_detail(&mut self, key: KeyCode) {
        let Some(open) = self.details.open.as_mut() else {
            return;
        };
        // Keyboard events may arrive before the first panel draw.
        let options = self.details.options.expect("enabled inspector");
        let width = usize::from(self.area.width.min(64).saturating_sub(2)).max(1);
        let height = usize::from(
            self.area
                .height
                .min(options.max_height.max(4))
                .saturating_sub(3),
        )
        .max(1);
        let maximum = wrapped_lines(&open.content.value, width)
            .count()
            .saturating_sub(height);
        open.scroll = match key {
            KeyCode::Home => 0,
            KeyCode::End => maximum,
            KeyCode::Up => open.scroll.saturating_sub(1),
            KeyCode::PageUp => open.scroll.saturating_sub(height),
            KeyCode::PageDown => open.scroll.saturating_add(height).min(maximum),
            _ => open.scroll.saturating_add(1).min(maximum),
        };
    }
    pub(super) fn render_cell_details(&mut self, buffer: &mut Buffer) {
        let Some(open) = self.details.open.as_ref() else {
            return;
        };
        let cell = (open.content.row, open.content.column);
        // Close snapshots when their value changes, disappears, or no longer overflows.
        if self.detail_content(cell).as_ref() != Some(&open.content) {
            self.clear_cell_details();
            return;
        }
        let options = self.details.options.expect("enabled inspector");
        let width = self.area.width.min(64);
        let height = self.area.height.min(options.max_height.max(4));
        let rect = Rect::new(
            self.area.right() - width,
            self.area.bottom() - height,
            width,
            height,
        );
        self.details.panel = rect;
        let open = self.details.open.as_ref().expect("open inspector");
        let style = self.style.cell.patch(self.style.header);
        for y in rect.y..rect.bottom() {
            for x in rect.x..rect.right() {
                buffer[(x, y)].reset();
                buffer[(x, y)].set_style(style);
            }
        }
        for x in rect.x..rect.right() {
            buffer[(x, rect.y)].set_symbol("─");
            buffer[(x, rect.bottom() - 1)].set_symbol("─");
        }
        for y in rect.y..rect.bottom() {
            buffer[(rect.x, y)].set_symbol("│");
            buffer[(rect.right() - 1, y)].set_symbol("│");
        }
        for (point, symbol) in [
            ((rect.x, rect.y), "┌"),
            ((rect.right() - 1, rect.y), "┐"),
            ((rect.x, rect.bottom() - 1), "└"),
            ((rect.right() - 1, rect.bottom() - 1), "┘"),
        ] {
            buffer[point].set_symbol(symbol);
        }
        buffer.set_stringn(
            rect.x + 1,
            rect.y + 1,
            format!("{} · Esc close", open.content.title),
            usize::from(width - 2),
            style,
        );
        let capacity = usize::from(height - 3);
        let mut lines =
            wrapped_lines(&open.content.value, usize::from(width - 2)).skip(open.scroll);
        for (line, text) in lines.by_ref().take(capacity).enumerate() {
            buffer.set_stringn(
                rect.x + 1,
                rect.y + 2 + line as u16,
                text,
                usize::from(width - 2),
                style,
            );
        }
        if open.scroll > 0 || lines.next().is_some() {
            buffer.set_stringn(
                rect.x + 1,
                rect.bottom() - 1,
                " ↑↓ / wheel scroll ",
                usize::from(width - 2),
                style,
            );
        }
    }
}
fn printable(text: &str) -> String {
    text.graphemes(true)
        .filter(|g| !g.chars().any(char::is_control))
        .collect()
}
// Borrow wrapped slices; rendering stops after the visible lines instead of
// allocating a string for every line of a potentially very large value.
fn wrapped_lines(text: &str, width: usize) -> impl Iterator<Item = &str> {
    let mut graphemes = text.grapheme_indices(true).peekable();
    std::iter::from_fn(move || {
        let (start, _) = *graphemes.peek()?;
        let mut used = 0;
        while let Some(&(index, grapheme)) = graphemes.peek() {
            let cells = UnicodeWidthStr::width(grapheme);
            if used > 0 && used + cells > width {
                return Some(&text[start..index]);
            }
            graphemes.next();
            used += cells;
        }
        Some(&text[start..])
    })
}

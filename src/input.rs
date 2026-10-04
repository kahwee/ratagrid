//! Event handling and layout-dependent navigation.
use super::*;
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind};
use unicode_segmentation::UnicodeSegmentation;

impl<T> Grid<T> {
    /// Route keyboard events only when this grid has application focus.
    /// Mouse input is handled only inside the last rendered area, except an active resize drag.
    /// Render again after an event before handling another mouse event.
    /// The result reports a change/action, not whether input was consumed: `None`
    /// can also mean a recognized navigation key made no change at a boundary.
    /// The application owns focus traversal; Tab cycles this grid's headers.
    pub(super) fn handle_grid_event(&mut self, event: &Event) -> Option<Action> {
        if self.search_draft.is_some() {
            match event {
                Event::Key(key) if key.kind != KeyEventKind::Release => {
                    if key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER)
                    {
                        return None;
                    }
                    match key.code {
                        KeyCode::Esc => {
                            self.search_draft = None;
                            return Some(Action::SearchEdited);
                        }
                        KeyCode::Enter => {
                            let query = self.search_draft.take().expect("search draft");
                            return self.set_search(query).or(Some(Action::SearchEdited));
                        }
                        KeyCode::Backspace => {
                            let draft = self.search_draft.as_mut().expect("search draft");
                            if let Some((start, _)) = draft.grapheme_indices(true).next_back() {
                                draft.truncate(start);
                            }
                            return Some(Action::SearchEdited);
                        }
                        KeyCode::Char(c) if !c.is_control() => {
                            self.search_draft.as_mut().expect("search draft").push(c);
                            return Some(Action::SearchEdited);
                        }
                        _ => return None,
                    }
                }
                Event::Paste(text) => {
                    self.search_draft
                        .as_mut()
                        .expect("search draft")
                        .extend(text.chars().filter(|c| !c.is_control()));
                    return Some(Action::SearchEdited);
                }
                // Keep the draft focused; mouse input cannot sort or navigate underneath it.
                Event::Mouse(_) => return None,
                _ => (),
            }
        }
        match event {
            Event::Mouse(mouse) => {
                if matches!(mouse.kind, MouseEventKind::Up(MouseButton::Left)) {
                    if self.drag.take().is_some() {
                        return self.set_hover(self.hit(mouse.column, mouse.row));
                    }
                    return None;
                }
                if matches!(mouse.kind, MouseEventKind::Drag(MouseButton::Left))
                    && let Some(drag) = self.drag
                {
                    let width = (i32::from(drag.width) + i32::from(mouse.column)
                        - i32::from(drag.x))
                    .clamp(4, i32::from(u16::MAX)) as u16;
                    if self.model.columns[drag.column].width == width {
                        return None;
                    }
                    self.model.columns[drag.column].width = width;
                    return Some(Action::ColumnResized {
                        column: drag.column,
                        width,
                    });
                }
                let hit = self.hit(mouse.column, mouse.row);
                if !self.contains(mouse.column, mouse.row) {
                    return self.set_hover(None);
                }
                match mouse.kind {
                    MouseEventKind::Moved => self.set_hover(hit),
                    MouseEventKind::Down(MouseButton::Left) => match hit {
                        Some(Hit::Resize(column)) => {
                            self.hover = None;
                            self.drag = Some(Drag {
                                column,
                                x: mouse.column,
                                width: self.model.columns[column].width,
                            });
                            None
                        }
                        Some(Hit::Header(column)) => {
                            self.header_focus = Some(column);
                            self.hover = None;
                            self.toggle_sort(column)
                                .or(Some(Action::HeaderFocused(column)))
                        }
                        Some(Hit::Row(position)) => {
                            let column = self
                                .layout
                                .iter()
                                .find(|c| {
                                    mouse.column >= c.clip_start && mouse.column < c.clip_end
                                })?
                                .column;
                            if mouse.modifiers.contains(KeyModifiers::SHIFT) {
                                self.extend_selection(position, Some(column))
                            } else if mouse.modifiers.contains(KeyModifiers::CONTROL) {
                                self.select_cell(position, column);
                                self.toggle_row_selection(self.model.index_at(position)?)
                            } else {
                                self.select_cell(position, column)
                            }
                        }
                        Some(Hit::Page(navigation)) => self.navigate(navigation),
                        Some(Hit::Retry) => self.reload_page(),
                        None => None,
                    },
                    MouseEventKind::ScrollDown if mouse.modifiers.contains(KeyModifiers::SHIFT) => {
                        self.scroll_columns(3)
                    }
                    MouseEventKind::ScrollUp if mouse.modifiers.contains(KeyModifiers::SHIFT) => {
                        self.scroll_columns(-3)
                    }
                    MouseEventKind::ScrollDown => self.scroll_rows(3),
                    MouseEventKind::ScrollUp => self.scroll_rows(-3),
                    MouseEventKind::ScrollLeft => self.scroll_columns(-3),
                    MouseEventKind::ScrollRight => self.scroll_columns(3),
                    _ => None,
                }
            }
            Event::Key(key) if key.kind != KeyEventKind::Release => {
                if key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key
                        .modifiers
                        .intersects(KeyModifiers::ALT | KeyModifiers::SUPER)
                {
                    match key.code {
                        KeyCode::Char('c' | 'C') => {
                            let target = if key.modifiers.contains(KeyModifiers::SHIFT) {
                                CopyTarget::SelectedRows
                            } else {
                                let (row, column) = self.cursor()?;
                                CopyTarget::Cell { row, column }
                            };
                            return self
                                .can_copy(target)
                                .then_some(Action::CopyRequested(target));
                        }
                        KeyCode::Char('a' | 'A') => return self.select_page_rows(),
                        KeyCode::Home => return self.navigate(Navigation::First),
                        KeyCode::End => return self.navigate(Navigation::Last),
                        KeyCode::PageUp => return self.navigate(Navigation::Previous),
                        KeyCode::PageDown => return self.navigate(Navigation::Next),
                        _ => (),
                    }
                }
                if key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER)
                {
                    return None;
                }
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    let range = self.row_range();
                    if !range.is_empty() {
                        let current = self.model.selected_position().filter(|p| range.contains(p));
                        let delta = match key.code {
                            KeyCode::Up => Some(-1),
                            KeyCode::Down => Some(1),
                            KeyCode::PageUp => Some(-(self.body_height().max(1) as isize)),
                            KeyCode::PageDown => Some(self.body_height().max(1) as isize),
                            _ => None,
                        };
                        let position = match key.code {
                            KeyCode::Home => Some(range.start),
                            KeyCode::End => Some(range.end - 1),
                            _ => delta.map(|delta| {
                                current.map_or(
                                    if delta < 0 {
                                        range.end - 1
                                    } else {
                                        range.start
                                    },
                                    |p| {
                                        p.saturating_add_signed(delta)
                                            .clamp(range.start, range.end - 1)
                                    },
                                )
                            }),
                        };
                        if let Some(position) = position {
                            return self.extend_selection(position, None);
                        }
                    }
                }
                match key.code {
                    KeyCode::Char('/') => Some(self.begin_search()),
                    KeyCode::F(5) => self.reload_page(),
                    KeyCode::Char(' ') if self.header_focus.is_none() => {
                        let position = self.model.selected_position()?;
                        if !self.row_range().contains(&position) {
                            return None;
                        }
                        self.toggle_row_selection(self.model.selected_index()?)
                    }
                    KeyCode::Tab | KeyCode::BackTab => {
                        let columns = self.visible_columns();
                        let n = columns.len();
                        if n == 0 {
                            return None;
                        }
                        let current = self
                            .header_focus
                            .and_then(|column| columns.iter().position(|&c| c == column));
                        let backwards = key.code == KeyCode::BackTab
                            || key.modifiers.contains(KeyModifiers::SHIFT);
                        let position = match (current, backwards) {
                            (Some(i), true) => (i + n - 1) % n,
                            (Some(i), false) => (i + 1) % n,
                            (None, true) => n - 1,
                            (None, false) => 0,
                        };
                        let column = columns[position];
                        self.header_focus = Some(column);
                        self.reveal_column(column);
                        Some(Action::HeaderFocused(column))
                    }
                    KeyCode::Enter | KeyCode::Char(' ') if self.header_focus.is_some() => {
                        self.hover = None;
                        self.toggle_sort(self.header_focus?)
                    }
                    KeyCode::Enter => self
                        .model
                        .selected_position()
                        .filter(|position| self.row_range().contains(position))
                        .and_then(|_| self.model.selected_index())
                        .map(Action::RowActivated),
                    KeyCode::Down | KeyCode::Char('j') => self.move_selection(1),
                    KeyCode::Up | KeyCode::Char('k') => self.move_selection(-1),
                    KeyCode::PageDown => self.move_selection(self.body_height().max(1) as isize),
                    KeyCode::PageUp => self.move_selection(-(self.body_height().max(1) as isize)),
                    KeyCode::Home => self.select_position(self.row_range().start),
                    KeyCode::End => self.select_position(self.row_range().end.saturating_sub(1)),
                    KeyCode::Char('[') => self.navigate(Navigation::Previous),
                    KeyCode::Char(']') => self.navigate(Navigation::Next),
                    KeyCode::Left | KeyCode::Right
                        if key.modifiers.contains(KeyModifiers::SHIFT) =>
                    {
                        self.scroll_columns(if key.code == KeyCode::Left { -3 } else { 3 })
                    }
                    KeyCode::Left | KeyCode::Char('h') => self.move_cursor(-1),
                    KeyCode::Right | KeyCode::Char('l') => self.move_cursor(1),
                    KeyCode::Char('+') | KeyCode::Char('=') => self.resize_focused(1),
                    KeyCode::Char('-') => self.resize_focused(-1),
                    KeyCode::Esc => {
                        let had_focus = self.header_focus.take().is_some();
                        self.drag = None;
                        if had_focus {
                            Some(Action::HeaderBlurred)
                        } else {
                            self.clear_row_selection()
                        }
                    }
                    _ => None,
                }
            }
            Event::Resize(_, _) => {
                self.hover = None;
                self.drag = None;
                None
            }
            _ => None,
        }
    }

    pub(super) fn body_height(&self) -> usize {
        usize::from(
            self.area
                .height
                .saturating_sub(1)
                .saturating_sub(u16::from(self.has_footer())),
        )
    }
    pub(super) fn has_footer(&self) -> bool {
        (self.pagination.is_some() || self.is_searching() || !self.search_query.is_empty())
            && self.area.height >= 2
    }
    pub(super) fn contains(&self, x: u16, y: u16) -> bool {
        x >= self.area.x && x < self.area.right() && y >= self.area.y && y < self.area.bottom()
    }
    pub(super) fn hit(&self, x: u16, y: u16) -> Option<Hit> {
        if !self.contains(x, y) {
            return None;
        }
        if self
            .retry_button
            .is_some_and(|rect| rect.contains((x, y).into()))
            && matches!(self.load_state(), LoadState::Error(_))
        {
            return Some(Hit::Retry);
        }
        if self.has_footer() && y == self.area.bottom() - 1 {
            if self.is_searching() {
                return None;
            }
            return self
                .page_buttons
                .iter()
                .find(|(rect, _, enabled)| *enabled && rect.contains((x, y).into()))
                .map(|(_, navigation, _)| Hit::Page(*navigation));
        }
        let layout = self
            .layout
            .iter()
            .find(|c| x >= c.clip_start && x < c.clip_end)?;
        if y == self.area.y {
            if i64::from(x) == layout.end - 1 {
                Some(Hit::Resize(layout.column))
            } else {
                Some(Hit::Header(layout.column))
            }
        } else {
            let range = self.row_range();
            let position = range.start + self.row_offset + usize::from(y - self.area.y - 1);
            (position < range.end).then_some(Hit::Row(position))
        }
    }
    pub(super) fn set_hover(&mut self, hit: Option<Hit>) -> Option<Action> {
        if self.hover == hit {
            return None;
        }
        self.hover = hit;
        Some(Action::HoverChanged)
    }
    pub(super) fn scroll_rows(&mut self, delta: isize) -> Option<Action> {
        let max = self.row_range().len().saturating_sub(self.body_height());
        let next = self.row_offset.saturating_add_signed(delta).min(max);
        if next == self.row_offset {
            return None;
        }
        self.row_offset = next;
        self.hover = None;
        Some(Action::Scrolled)
    }
    pub(super) fn scroll_columns(&mut self, delta: isize) -> Option<Action> {
        let max = self.max_column_offset();
        let next = self.column_offset.saturating_add_signed(delta).min(max);
        if next == self.column_offset {
            return None;
        }
        self.column_offset = next;
        self.hover = None;
        Some(Action::Scrolled)
    }
    pub(super) fn reveal_column(&mut self, column: usize) {
        if !self.is_column_visible(column) || self.is_column_pinned(column) {
            return;
        }
        let start: usize = self
            .column_order
            .iter()
            .copied()
            .filter(|&c| self.column_visible[c] && !self.column_pinned[c])
            .take_while(|&c| c != column)
            .map(|c| usize::from(self.model.columns[c].width))
            .sum();
        let end = start + usize::from(self.model.columns[column].width);
        let width = self.scrollable_viewport_width();
        if start < self.column_offset || end - start > width {
            self.column_offset = start;
        } else if end > self.column_offset + width {
            self.column_offset = end.saturating_sub(width);
        }
        self.column_offset = self.column_offset.min(self.max_column_offset());
    }

    fn move_cursor(&mut self, delta: isize) -> Option<Action> {
        let columns = self.visible_columns();
        let last = columns.len().checked_sub(1)?;
        if let Some(column) = self.header_focus {
            let current = columns.iter().position(|&c| c == column)?;
            let next = columns[current.saturating_add_signed(delta).min(last)];
            let previous_offset = self.column_offset;
            self.reveal_column(next);
            self.hover = None;
            self.header_focus = Some(next);
            return if next != column {
                Some(Action::HeaderFocused(next))
            } else {
                (previous_offset != self.column_offset).then_some(Action::Scrolled)
            };
        }
        let range = self.row_range();
        if range.is_empty() {
            return None;
        }
        let position = self.model.selected_position().filter(|p| range.contains(p));
        let current = self
            .cursor_column
            .and_then(|column| columns.iter().position(|&c| c == column))
            .unwrap_or(0);
        let column = columns[if position.is_some() {
            current.saturating_add_signed(delta).min(last)
        } else {
            current
        }];
        self.select_cell(position.unwrap_or(range.start), column)
    }

    fn select_cell(&mut self, position: usize, column: usize) -> Option<Action> {
        let previous = self.cursor();
        self.cursor_column = Some(column);
        let action = self.select_position(position);
        if matches!(action, None | Some(Action::Scrolled))
            && let Some((row, column)) = self.cursor()
            && previous != Some((row, column))
        {
            Some(Action::CursorMoved { row, column })
        } else {
            action
        }
    }

    pub(super) fn move_selection(&mut self, delta: isize) -> Option<Action> {
        let range = self.row_range();
        if range.is_empty() {
            return None;
        }
        let next = match self.model.selected_position().filter(|p| range.contains(p)) {
            Some(position) => position.saturating_add_signed(delta),
            None if delta < 0 => range.end - 1,
            None => range.start,
        }
        .clamp(range.start, range.end - 1);
        self.select_position(next)
    }
    pub(super) fn select_position(&mut self, position: usize) -> Option<Action> {
        let range = self.row_range();
        if !range.contains(&position) {
            return None;
        }
        let previous_offsets = (self.row_offset, self.column_offset);
        let columns = self.visible_columns();
        if let Some(&first) = columns.first() {
            let column = self
                .cursor_column
                .or(self.header_focus)
                .filter(|c| columns.contains(c))
                .unwrap_or(first);
            self.cursor_column = Some(column);
            self.reveal_column(column);
        }
        let had_focus = self.header_focus.take().is_some();
        let changed = self.model.select(position);
        self.selection_anchor = self.model.selected_index();
        self.range_base = None;
        self.reveal_selected_row();
        self.hover = None;
        if changed {
            Some(Action::SelectionChanged(
                self.model.selected_index().expect("selected row"),
            ))
        } else {
            had_focus.then_some(Action::HeaderBlurred).or_else(|| {
                (previous_offsets != (self.row_offset, self.column_offset))
                    .then_some(Action::Scrolled)
            })
        }
    }

    pub(super) fn reveal_selected_row(&mut self) {
        let range = self.row_range();
        let Some(position) = self.model.selected_position().filter(|p| range.contains(p)) else {
            return;
        };
        let local_position = position - range.start;
        let height = self.body_height().max(1);
        if local_position < self.row_offset {
            self.row_offset = local_position;
        } else if local_position >= self.row_offset.saturating_add(height) {
            self.row_offset = local_position.saturating_sub(height - 1);
        }
    }

    pub(super) fn resize_focused(&mut self, delta: i16) -> Option<Action> {
        let column = self.header_focus?;
        let width = self.model.columns[column]
            .width
            .saturating_add_signed(delta)
            .max(4);
        if width == self.model.columns[column].width {
            return None;
        }
        self.model.columns[column].width = width;
        Some(Action::ColumnResized { column, width })
    }
}

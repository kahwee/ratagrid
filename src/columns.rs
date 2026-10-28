//! Column presentation keeps original indices stable for sorting and actions.
use super::*;

/// One column in a saved layout, in display order (including hidden columns).
/// The insertion index and title identify the original schema. Applications
/// should also version their saved schema if column meanings change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnState {
    pub column: usize,
    pub title: String,
    pub width: u16,
    pub visible: bool,
    pub pinned: bool,
}

/// Plain data for application-owned persistence; no storage or serialization
/// dependency is imposed. Columns must match the receiving grid's schema.
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct GridLayout {
    pub columns: Vec<ColumnState>,
}

/// A saved layout was rejected without changing the grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutError {
    /// Missing, duplicate, or out-of-range column indices.
    InvalidColumns,
    /// The saved title differs from the current column at this insertion index.
    SchemaMismatch { column: usize },
    /// Widths must be at least four terminal cells.
    InvalidWidth { column: usize },
}

impl std::fmt::Display for LayoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidColumns => f.write_str("layout must contain each column exactly once"),
            Self::SchemaMismatch { column } => {
                write!(f, "layout schema differs at column {column}")
            }
            Self::InvalidWidth { column } => {
                write!(f, "column {column} width must be at least four")
            }
        }
    }
}
impl std::error::Error for LayoutError {}

/// Invalid column order. Supply each original column index exactly once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColumnOrderError;

impl std::fmt::Display for ColumnOrderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("column order must contain each original column index exactly once")
    }
}
impl std::error::Error for ColumnOrderError {}

impl<T> Grid<T> {
    /// Capture widths, visibility, order and pinning for application persistence.
    pub fn save_layout(&self) -> GridLayout {
        GridLayout {
            columns: self
                .column_order
                .iter()
                .map(|&column| ColumnState {
                    column,
                    title: self.model.columns[column].title.clone(),
                    width: self.model.columns[column].width,
                    visible: self.column_visible[column],
                    pinned: self.column_pinned[column],
                })
                .collect(),
        }
    }

    /// Validate the complete snapshot before applying it. A rejected snapshot
    /// preserves widths, order, focus and the current interaction state.
    /// A successful restore closes cell details and cancels resizing; render
    /// again before handling mouse input. Row data, sorting and selection remain.
    pub fn restore_layout(&mut self, layout: &GridLayout) -> Result<(), LayoutError> {
        if layout.columns.len() != self.model.columns.len() {
            return Err(LayoutError::InvalidColumns);
        }
        let mut seen = vec![false; self.model.columns.len()];
        for state in &layout.columns {
            let Some(present) = seen.get_mut(state.column) else {
                return Err(LayoutError::InvalidColumns);
            };
            if *present {
                return Err(LayoutError::InvalidColumns);
            }
            *present = true;
            if state.title != self.model.columns[state.column].title {
                return Err(LayoutError::SchemaMismatch {
                    column: state.column,
                });
            }
            if state.width < 4 {
                return Err(LayoutError::InvalidWidth {
                    column: state.column,
                });
            }
        }
        for state in &layout.columns {
            self.model.columns[state.column].width = state.width;
            self.column_visible[state.column] = state.visible;
            self.column_pinned[state.column] = state.pinned;
        }
        self.column_order = layout.columns.iter().map(|state| state.column).collect();
        self.columns_changed();
        self.column_offset = self.column_offset.min(self.max_column_offset());
        Ok(())
    }

    /// Original column indices in configured order, including hidden columns.
    pub fn column_order(&self) -> &[usize] {
        &self.column_order
    }

    /// Visible columns in display order: pinned columns first, then scrollable columns.
    pub fn visible_columns(&self) -> Vec<usize> {
        [true, false]
            .into_iter()
            .flat_map(|pinned| {
                self.column_order.iter().copied().filter(move |&column| {
                    self.column_visible[column] && self.column_pinned[column] == pinned
                })
            })
            .collect()
    }

    pub fn is_column_visible(&self, column: usize) -> bool {
        self.column_visible.get(column).copied().unwrap_or(false)
    }

    pub fn is_column_pinned(&self, column: usize) -> bool {
        self.column_pinned.get(column).copied().unwrap_or(false)
    }

    /// Show/hide a column. All columns may be hidden; row navigation still works.
    /// Returns false for invalid indices or an unchanged setting.
    pub fn set_column_visible(&mut self, column: usize, visible: bool) -> bool {
        let Some(value) = self.column_visible.get_mut(column) else {
            return false;
        };
        if *value == visible {
            return false;
        }
        *value = visible;
        self.columns_changed();
        true
    }

    /// Freeze a column at the left edge. Pinning does not change its original index.
    /// Pinned columns use configured order and may consume the entire viewport.
    pub fn set_column_pinned(&mut self, column: usize, pinned: bool) -> bool {
        let Some(value) = self.column_pinned.get_mut(column) else {
            return false;
        };
        if *value == pinned {
            return false;
        }
        *value = pinned;
        self.columns_changed();
        true
    }

    /// Reorder using original indices, including hidden columns. Invalid input
    /// leaves layout and focus unchanged. Length, bounds, and duplicates are
    /// checked in one pass. Pinned columns are displayed first.
    pub fn set_column_order(&mut self, order: Vec<usize>) -> Result<(), ColumnOrderError> {
        if order.len() != self.model.columns.len() {
            return Err(ColumnOrderError);
        }
        let mut seen = vec![false; order.len()];
        for &column in &order {
            let Some(present) = seen.get_mut(column) else {
                return Err(ColumnOrderError);
            };
            if *present {
                return Err(ColumnOrderError);
            }
            *present = true;
        }
        self.column_order = order;
        self.columns_changed();
        Ok(())
    }

    fn columns_changed(&mut self) {
        self.hover = None;
        self.clear_cell_details();
        self.drag = None;
        self.layout.clear();
        let first = self.visible_columns().first().copied();
        if self
            .header_focus
            .is_some_and(|column| !self.is_column_visible(column))
        {
            self.header_focus = first;
        }
        if self
            .cursor_column
            .is_some_and(|column| !self.is_column_visible(column))
            || (self.cursor_column.is_none() && self.model.selected_index().is_some())
        {
            self.cursor_column = first;
        }
        if let Some(column) = self.header_focus.or(self.cursor_column) {
            self.reveal_column(column);
        }
    }

    pub(super) fn pinned_width(&self) -> usize {
        self.column_order
            .iter()
            .copied()
            .filter(|&column| self.column_visible[column] && self.column_pinned[column])
            .map(|column| usize::from(self.model.columns[column].width))
            .sum()
    }

    pub(super) fn scrollable_width(&self) -> usize {
        self.column_order
            .iter()
            .copied()
            .filter(|&column| self.column_visible[column] && !self.column_pinned[column])
            .map(|column| usize::from(self.model.columns[column].width))
            .sum()
    }

    pub(super) fn scrollable_viewport_width(&self) -> usize {
        usize::from(self.area.width).saturating_sub(self.pinned_width())
    }

    pub(super) fn max_column_offset(&self) -> usize {
        let width = self.scrollable_viewport_width();
        if width == 0 {
            0
        } else {
            self.scrollable_width().saturating_sub(width)
        }
    }
}

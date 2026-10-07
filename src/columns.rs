//! Column presentation keeps original indices stable for sorting and actions.
use super::*;

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
    /// leaves layout and focus unchanged. Pinned columns are displayed first.
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

//! Structural changes to owned datasets; remote sources retain page ownership.
use super::*;

/// Structural row changes must be performed in the external source instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExternalRowsError;

impl std::fmt::Display for ExternalRowsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("append or remove rows in the external source, then reload the page")
    }
}
impl std::error::Error for ExternalRowsError {}

impl<T> Grid<T> {
    /// Append owned records without replacing existing records or selection.
    /// Returns the number added, including rows hidden by search/filtering.
    /// Client pagination totals update immediately; the current page is retained.
    /// An external grid rejects this before consuming the iterator.
    /// Render again before handling mouse events.
    ///
    /// ```
    /// use ratagrid::{Column, Grid};
    /// let mut grid = Grid::new(vec![Column::new("Value", 8, |n: &i32| n.to_string())], vec![1]);
    /// assert_eq!(grid.append_rows([2, 3]).unwrap(), 2);
    /// assert_eq!(grid.model().rows(), &[1, 2, 3]);
    /// ```
    pub fn append_rows(
        &mut self,
        rows: impl IntoIterator<Item = T>,
    ) -> Result<usize, ExternalRowsError> {
        if self.is_external() {
            return Err(ExternalRowsError);
        }
        let added = self.model.append_rows(rows);
        if added > 0 {
            self.owned_rows_changed();
        }
        Ok(added)
    }

    /// Remove owned records that fail `keep`, preserving surviving selections.
    /// The predicate sees every resident record, including filtered-out records,
    /// once in insertion order. Returns the number removed. Row indices compact;
    /// discard application-cached indices afterward. Existing flashes are cleared
    /// on removal so they cannot move to a different record. Client pages clamp
    /// immediately. No removals leave interaction state and flashes unchanged.
    /// An external grid rejects this without invoking `keep`.
    /// Render again before handling mouse events.
    pub fn retain_rows(
        &mut self,
        keep: impl FnMut(&T) -> bool,
    ) -> Result<usize, ExternalRowsError> {
        if self.is_external() {
            return Err(ExternalRowsError);
        }
        let removed = self.model.retain_rows(keep);
        if removed > 0 {
            self.clear_animations();
            self.owned_rows_changed();
        }
        Ok(removed)
    }

    fn owned_rows_changed(&mut self) {
        self.selection_anchor = self.model.selected_index();
        self.range_base = None;
        self.clamp_page();
        self.hover = None;
        self.clear_cell_details();
    }
}

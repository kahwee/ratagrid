//! Search, bulk selection, copy requests, and source lifecycle.
use super::*;
use std::collections::BTreeSet;

/// What to format for an application-owned clipboard operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyTarget {
    Cell {
        row: usize,
        column: usize,
    },
    Row(usize),
    /// Marked rows in filtered/sorted order, or the active row if no rows are marked.
    /// Hidden columns are omitted. Owned marks may span client pages.
    SelectedRows,
}

/// Current display state. Errors belong to the current external request only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadState<'a> {
    Loading,
    Error(&'a str),
    Empty,
    Ready,
}

/// Typed predicates cannot be executed against records held by an external source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExternalFilterError;
impl std::fmt::Display for ExternalFilterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("use set_search and filter in the source for an external grid")
    }
}
impl std::error::Error for ExternalFilterError {}

impl<T> Grid<T> {
    /// Committed query. Capture this with each external PageRequested action;
    /// a response still carries its original request to reject stale results.
    pub fn search_query(&self) -> &str {
        &self.search_query
    }

    /// Draft text while `/` search entry owns keyboard input.
    pub fn search_draft(&self) -> Option<&str> {
        self.search_draft.as_deref()
    }

    pub fn is_searching(&self) -> bool {
        self.search_draft.is_some()
    }

    /// Open query entry. Enter commits; Escape cancels without changing the query.
    pub fn begin_search(&mut self) -> Action {
        self.search_draft = Some(self.search_query.clone());
        self.hover = None;
        self.clear_cell_details();
        self.drag = None;
        Action::SearchEdited
    }

    /// Search all defined formatted columns (Unicode lowercase substring) for
    /// owned data. External data emits a new request, resets to page one, and
    /// discards the old count; the application applies the query globally.
    pub fn set_search(&mut self, query: impl Into<String>) -> Option<Action> {
        let query: String = query
            .into()
            .chars()
            .filter(|c| !crate::text::is_unsafe(*c))
            .collect();
        if query == self.search_query {
            return None;
        }
        self.search_query = query;
        self.row_offset = 0;
        self.hover = None;
        self.clear_cell_details();
        self.drag = None;
        self.range_base = None;
        self.selection_anchor = None;
        if self.is_external() {
            let p = self.pagination.as_mut()?;
            p.page = 0;
            p.total = None;
            self.page_action()
        } else {
            self.model.set_search(self.search_query.clone());
            if let Some(p) = self.pagination.as_mut() {
                p.page = 0;
            }
            Some(Action::FilterChanged)
        }
    }

    /// Filter owned records by a typed predicate, combined with the text query.
    /// Returns an error without mutation for an external grid.
    pub fn set_filter(
        &mut self,
        filter: impl Fn(&T) -> bool + 'static,
    ) -> Result<Action, ExternalFilterError> {
        if self.is_external() {
            return Err(ExternalFilterError);
        }
        self.model.set_filter(filter);
        self.filter_changed();
        Ok(Action::FilterChanged)
    }

    pub fn clear_filter(&mut self) -> Result<Action, ExternalFilterError> {
        if self.is_external() {
            return Err(ExternalFilterError);
        }
        self.model.clear_filter();
        self.filter_changed();
        Ok(Action::FilterChanged)
    }

    fn filter_changed(&mut self) {
        self.row_offset = 0;
        self.hover = None;
        self.clear_cell_details();
        self.drag = None;
        self.selection_anchor = None;
        self.range_base = None;
        if let Some(p) = self.pagination.as_mut() {
            p.page = 0;
        }
    }

    /// Mark all filtered rows in the current page, preserving marks on other pages.
    pub fn select_page_rows(&mut self) -> Option<Action> {
        let mut indices: BTreeSet<_> = self.model.selected_indices().collect();
        indices.extend(self.row_range().filter_map(|p| self.model.index_at(p)));
        self.range_base = None;
        self.model
            .set_selected_indices(indices)
            .then_some(Action::RowsSelected)
    }

    pub fn clear_row_selection(&mut self) -> Option<Action> {
        self.range_base = None;
        self.model
            .clear_row_selection()
            .then_some(Action::RowsSelected)
    }

    /// Toggle a resident insertion index without moving the cursor.
    pub fn toggle_row_selection(&mut self, index: usize) -> Option<Action> {
        self.range_base = None;
        self.model
            .toggle_row_selection(index)
            .then_some(Action::RowsSelected)
    }

    pub(super) fn extend_selection(
        &mut self,
        position: usize,
        column: Option<usize>,
    ) -> Option<Action> {
        let range = self.row_range();
        if !range.contains(&position) {
            return None;
        }
        let anchor_index = self.selection_anchor.or(self.model.selected_index());
        let anchor = anchor_index
            .and_then(|index| self.model.order.iter().position(|&i| i == index))
            .filter(|p| range.contains(p))
            .unwrap_or(position);
        let anchor_index = self.model.index_at(anchor);
        let base = self
            .range_base
            .take()
            .unwrap_or_else(|| self.model.selected_indices().collect());
        if let Some(column) = column {
            self.cursor_column = Some(column);
        }
        self.select_position(position);
        let mut indices = base.clone();
        indices.extend(
            (anchor.min(position)..=anchor.max(position)).filter_map(|p| self.model.index_at(p)),
        );
        self.model.set_selected_indices(indices);
        self.selection_anchor = anchor_index;
        self.range_base = Some(base);
        Some(Action::RowsSelected)
    }

    /// Full formatted text, never viewport-clipped. Rows use tabs between visible
    /// columns and newlines between records. Terminal controls and bidirectional
    /// formatting controls within cells are removed, preventing separator injection.
    /// Clipboard access remains the application's responsibility.
    pub fn copy_text(&self, target: CopyTarget) -> Option<String> {
        fn plain(text: String) -> String {
            text.chars()
                .filter(|c| !crate::text::is_unsafe(*c))
                .collect()
        }
        let columns = self.visible_columns();
        let row_text = |index| {
            let row = self.model.rows().get(index)?;
            Some(
                columns
                    .iter()
                    .map(|&column| plain((self.model.columns[column].format)(row)))
                    .collect::<Vec<_>>()
                    .join("\t"),
            )
        };
        match target {
            CopyTarget::Cell { row, column } => {
                if !self.is_column_visible(column) {
                    return None;
                }
                Some(plain((self.model.columns.get(column)?.format)(
                    self.model.rows().get(row)?,
                )))
            }
            CopyTarget::Row(row) => {
                if columns.is_empty() {
                    None
                } else {
                    row_text(row)
                }
            }
            CopyTarget::SelectedRows => {
                let marked = self.model.selected_indices().next().is_some();
                let indices: Vec<_> = if marked {
                    self.model
                        .order
                        .iter()
                        .copied()
                        .filter(|&i| self.model.is_row_selected(i))
                        .collect()
                } else {
                    vec![self.cursor()?.0]
                };
                if indices.is_empty() || columns.is_empty() {
                    return None;
                }
                Some(
                    indices
                        .into_iter()
                        .filter_map(row_text)
                        .collect::<Vec<_>>()
                        .join("\n"),
                )
            }
        }
    }

    pub(super) fn can_copy(&self, target: CopyTarget) -> bool {
        match target {
            CopyTarget::Cell { row, column } => {
                self.is_column_visible(column) && row < self.model.rows().len()
            }
            CopyTarget::Row(row) => {
                !self.visible_columns().is_empty() && row < self.model.rows().len()
            }
            CopyTarget::SelectedRows => {
                !self.visible_columns().is_empty()
                    && if self.model.selected_indices().next().is_some() {
                        self.model
                            .order
                            .iter()
                            .any(|&index| self.model.is_row_selected(index))
                    } else {
                        self.cursor().is_some()
                    }
            }
        }
    }

    pub fn load_state(&self) -> LoadState<'_> {
        if let Some(error) = &self.page_error {
            LoadState::Error(error)
        } else if self.page_state().is_some_and(|p| p.loading) {
            LoadState::Loading
        } else if self.model.visible_len() == 0 {
            LoadState::Empty
        } else {
            LoadState::Ready
        }
    }

    /// Complete a pending external request with a source error. Stale, duplicate,
    /// or foreign results are rejected. Retry via reload_page (or F5 / Retry button)
    /// creates a new request; late data for the failed request cannot overwrite it.
    pub fn set_page_error(
        &mut self,
        request: PageRequest,
        message: impl Into<String>,
    ) -> Result<(), PageError> {
        let current = self.page_request().ok_or(PageError::NotExternal)?;
        if current != request || !self.page_state().is_some_and(|p| p.loading) {
            return Err(PageError::StaleResponse);
        }
        let message: String = message
            .into()
            .chars()
            .filter(|c| !crate::text::is_unsafe(*c))
            .collect();
        self.page_error = Some(if message.is_empty() {
            "Unable to load records".into()
        } else {
            message
        });
        let p = self.pagination.as_mut().expect("external pagination");
        p.loading = false;
        p.has_next = false;
        self.hover = None;
        self.clear_cell_details();
        Ok(())
    }
}

//! Grid pagination: request validation, resident pages and navigation.
use super::*;
use std::{num::NonZeroUsize, ops::Range};

impl<T> Grid<T> {
    /// Enable paging an owned dataset. The full dataset stays resident and is sorted globally.
    pub fn with_pagination(mut self, page_size: NonZeroUsize) -> Self {
        self.set_page_size(page_size);
        self
    }

    /// Construct a grid that stores only one page. Fetch `page_request()` before
    /// its first render, then handle `Action::PageRequested` to fetch subsequent pages.
    /// Pass a row count (or `Some(count)`) for numbered pages, or `None` when the
    /// total is unknown. Unknown totals allow forward navigation after a full page
    /// arrives and stop after a short or empty page; last-page navigation is unavailable.
    ///
    /// ```
    /// use ratagrid::{Column, Grid};
    /// use std::num::NonZeroUsize;
    /// let columns = vec![Column::new("ID", 12, |id: &u64| id.to_string())
    ///     .sortable_external()];
    /// let mut grid = Grid::new_paged(columns, 100_000_000, NonZeroUsize::new(50).unwrap());
    /// let request = grid.page_request().unwrap();
    /// // Fetch using request.offset(), request.page_size and request.sort.
    /// let rows = (request.offset() as u64..).take(request.page_size.get()).collect();
    /// grid.set_page_data(request, rows).unwrap();
    /// assert_eq!(grid.page_state().unwrap().loaded_rows, 50);
    /// ```
    pub fn new_paged(
        columns: Vec<Column<T>>,
        total_rows: impl Into<Option<usize>>,
        page_size: NonZeroUsize,
    ) -> Self {
        let mut grid = Self::new(columns, vec![]);
        grid.pagination = Some(Pagination {
            mode: PaginationMode::External,
            page: 0,
            size: page_size,
            total: total_rows.into(),
            has_next: false,
            revision: pagination::next_revision(),
            loading: true,
            cursors: Vec::new(),
        });
        grid
    }

    /// Construct a grid whose source uses opaque cursor/keyset pagination.
    /// Fetch [`Self::cursor_page_request`] initially, then handle
    /// [`Action::CursorPageRequested`]. The grid performs no database I/O.
    /// Only the current page is resident; previous request tokens are retained.
    /// Totals and last-page navigation are unavailable in this mode.
    pub fn new_cursor_paged(columns: Vec<Column<T>>, page_size: NonZeroUsize) -> Self {
        let mut grid = Self::new_paged(columns, None, page_size);
        let p = grid.pagination.as_mut().expect("pagination");
        p.mode = PaginationMode::Cursor;
        p.reset_cursors();
        grid
    }

    /// Current cursor request. Check `page_state().loading` for pending work.
    /// Capture this and `search_query()` together before dispatching a worker.
    pub fn cursor_page_request(&self) -> Option<CursorPageRequest> {
        let p = self.pagination.as_ref()?;
        (p.mode == PaginationMode::Cursor).then(|| CursorPageRequest {
            page: p.page,
            page_size: p.size,
            sort: self.model.sort(),
            revision: p.revision,
            cursor: p.cursors[p.page].clone(),
        })
    }

    /// Accept one cursor response, preserving source order. `next_cursor: None`
    /// marks the end, even for a full page. A token permits Next even for a short
    /// or empty page. At most `page_size` rows may be returned. Revisiting or
    /// reloading a page replaces its continuation and discards later history.
    /// Stale, foreign, duplicate, and oversized responses leave the grid unchanged.
    pub fn set_cursor_page_data(
        &mut self,
        request: CursorPageRequest,
        rows: Vec<T>,
        next_cursor: Option<String>,
    ) -> Result<(), PageError> {
        self.validate_cursor_request(&request)?;
        if rows.len() > request.page_size.get() {
            return Err(PageError::WrongRowCount {
                expected: request.page_size.get(),
                actual: rows.len(),
            });
        }
        let p = self.pagination.as_mut().expect("cursor pagination");
        p.cursors.truncate(p.page + 1);
        let has_next = next_cursor.is_some();
        if has_next {
            p.cursors.push(next_cursor);
        }
        self.accept_page_rows(rows, has_next);
        Ok(())
    }

    pub(super) fn validate_cursor_request(
        &self,
        request: &CursorPageRequest,
    ) -> Result<(), PageError> {
        let current = self.cursor_page_request().ok_or(PageError::NotCursor)?;
        if current != *request || !self.page_state().expect("cursor pagination").loading {
            return Err(PageError::StaleResponse);
        }
        Ok(())
    }

    /// Restart cursor traversal from the beginning with a fresh request. Use
    /// after application-owned filters change or when a source token expires.
    /// Sorting, search and page-size changes already restart automatically.
    pub fn restart_cursor_pagination(&mut self) -> Option<Action> {
        let p = self.pagination.as_mut()?;
        if p.mode != PaginationMode::Cursor {
            return None;
        }
        p.page = 0;
        p.reset_cursors();
        self.page_action()
    }

    pub fn page_state(&self) -> Option<PageState> {
        let p = self.pagination.as_ref()?;
        let total = if p.mode == PaginationMode::Client {
            Some(self.model.visible_len())
        } else {
            p.total
        };
        Some(PageState {
            mode: p.mode,
            page: p.page,
            page_size: p.size,
            page_count: total.map(|total| p.count(total)),
            has_next_page: total.map_or(!p.loading && p.has_next, |total| {
                p.page < p.count(total) - 1
            }) && (p.mode == PaginationMode::Cursor
                || p.page < (usize::MAX - 1) / p.size.get()),
            total_rows: total,
            loaded_rows: self.model.rows().len(),
            loading: p.loading,
        })
    }

    /// Current offset-based external request, including after acceptance.
    /// Cursor mode returns `None`; use [`Self::cursor_page_request`]. Check
    /// `page_state().loading` for pending work; use `reload_page()` for a fresh request.
    pub fn page_request(&self) -> Option<PageRequest> {
        let p = self.pagination.as_ref()?;
        (p.mode == PaginationMode::External).then_some(PageRequest {
            page: p.page,
            page_size: p.size,
            sort: self.model.sort(),
            revision: p.revision,
        })
    }

    /// Accept the current request's complete page, in the globally sorted order returned
    /// by your source. No local sorting is performed. A request is accepted once;
    /// foreign, obsolete, duplicate, or invalid responses leave the grid unchanged.
    /// With a known total, the length must exactly match the expected page. With an
    /// unknown total, accept up to `page_size` rows: a full page enables Next, while
    /// a short or empty page marks the end. An exact multiple requires an empty
    /// response on the following page to discover the end.
    pub fn set_page_data(&mut self, request: PageRequest, rows: Vec<T>) -> Result<(), PageError> {
        let current = self.page_request().ok_or(PageError::NotExternal)?;
        if current != request {
            return Err(PageError::StaleResponse);
        }
        let state = self.page_state().expect("external pagination");
        if !state.loading {
            return Err(PageError::StaleResponse);
        }
        let expected = state.total_rows.map_or(request.page_size.get(), |total| {
            total
                .saturating_sub(request.offset())
                .min(request.page_size.get())
        });
        if rows.len() > expected || (state.total_rows.is_some() && rows.len() != expected) {
            return Err(PageError::WrongRowCount {
                expected,
                actual: rows.len(),
            });
        }
        let has_next = rows.len() == request.page_size.get();
        self.accept_page_rows(rows, has_next);
        Ok(())
    }

    fn accept_page_rows(&mut self, rows: Vec<T>, has_next: bool) {
        self.model.replace_rows_in_order(rows);
        self.clear_animations();
        let p = self.pagination.as_mut().expect("external pagination");
        p.loading = false;
        p.has_next = has_next;
        self.page_error = None;
        self.selection_anchor = self.model.selected_index();
        self.range_base = None;
        self.row_offset = 0;
        self.reveal_selected_row();
        self.hover = None;
        self.clear_cell_details();
    }

    /// Enable client pagination or change an existing page size. Resets to page one.
    /// External grids emit a fresh request and discard the previous page.
    pub fn set_page_size(&mut self, page_size: NonZeroUsize) -> Option<Action> {
        if self
            .pagination
            .as_ref()
            .is_some_and(|p| p.size == page_size)
        {
            return None;
        }
        if let Some(p) = self.pagination.as_mut() {
            p.size = page_size;
            p.page = 0;
            p.reset_cursors();
        } else {
            self.pagination = Some(Pagination {
                mode: PaginationMode::Client,
                page: 0,
                size: page_size,
                total: None,
                has_next: false,
                revision: 0,
                loading: false,
                cursors: Vec::new(),
            });
        }
        self.page_action()
    }

    /// Disable client pagination. External pagination cannot be disabled without fetching all records.
    pub fn disable_pagination(&mut self) -> bool {
        if self.pagination.is_none() || self.is_external() {
            return false;
        }
        self.pagination = None;
        self.row_offset = 0;
        self.hover = None;
        self.clear_cell_details();
        true
    }

    /// Go to a zero-based page, clamped to the last page. Selection persists for
    /// client pages; external pages discard records while retaining a configured
    /// selected ID until the next accepted response.
    /// With unknown totals, forward jumps are clamped to the next page, available
    /// only after a full offset response or a cursor response with a next token.
    /// Cursor mode revisits earlier pages using retained request tokens; it never
    /// seeks to unvisited pages or emits an offset request. Previous pages and
    /// page zero remain accessible.
    pub fn set_page(&mut self, page: usize) -> Option<Action> {
        let state = self.page_state()?;
        let last = state.page_count.map_or(
            state.page.saturating_add(usize::from(state.has_next_page)),
            |count| count - 1,
        );
        let page = page.min(last);
        if page == state.page {
            return None;
        }
        self.pagination.as_mut()?.page = page;
        self.page_action()
    }

    /// Update an offset-based external source's total count (for example after filtering).
    /// Cursor grids ignore this operation; counts do not enable random access.
    /// A known count clamps the current page. `None` removes the count and keeps the
    /// current page. Either change emits a new request, invalidating prior responses.
    pub fn set_total_rows(&mut self, total_rows: impl Into<Option<usize>>) -> Option<Action> {
        let total_rows = total_rows.into();
        if self.pagination.as_ref()?.mode != PaginationMode::External {
            return None;
        }
        let p = self.pagination.as_mut()?;
        if p.total == total_rows {
            return None;
        }
        p.total = total_rows;
        if let Some(total) = total_rows {
            p.page = p.page.min(p.count(total) - 1);
        }
        self.page_action()
    }

    /// Retry/reload the current external page, invalidating any earlier response.
    pub fn reload_page(&mut self) -> Option<Action> {
        if !self.is_external() {
            return None;
        }
        self.page_action()
    }

    pub(super) fn is_external(&self) -> bool {
        self.pagination
            .as_ref()
            .is_some_and(|p| p.mode != PaginationMode::Client)
    }
    pub(super) fn page_action(&mut self) -> Option<Action> {
        self.row_offset = 0;
        self.hover = None;
        self.clear_cell_details();
        self.drag = None;
        self.selection_anchor = None;
        self.range_base = None;
        self.page_error = None;
        if self.is_external() {
            let p = self.pagination.as_mut()?;
            p.revision = pagination::next_revision();
            p.loading = true;
            p.has_next = false;
            self.model.clear_rows_for_request();
            self.clear_animations();
            if let Some(request) = self.cursor_page_request() {
                Some(Action::CursorPageRequested(request))
            } else {
                Some(Action::PageRequested(self.page_request()?))
            }
        } else {
            Some(Action::PageChanged(self.page_state()?))
        }
    }
    pub(super) fn clamp_page(&mut self) {
        if let Some(state) = self.page_state()
            && let Some(count) = state.page_count
        {
            self.pagination.as_mut().expect("pagination").page = state.page.min(count - 1);
        }
    }
    pub(super) fn row_range(&self) -> Range<usize> {
        let len = self.model.visible_len();
        match self.page_state() {
            Some(p) if p.mode == PaginationMode::Client => {
                let start = p.page.saturating_mul(p.page_size.get()).min(len);
                start..start.saturating_add(p.page_size.get()).min(len)
            }
            _ => 0..len,
        }
    }
    pub(super) fn navigate(&mut self, navigation: Navigation) -> Option<Action> {
        let state = self.page_state()?;
        let page = match navigation {
            Navigation::First => 0,
            Navigation::Previous => state.page.saturating_sub(1),
            Navigation::Next => state.page.saturating_add(1),
            Navigation::Last => state.page_count? - 1,
        };
        self.set_page(page)
    }
}

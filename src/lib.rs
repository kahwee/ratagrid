//! A mouse-first data grid for Ratatui.
//!
//! Render [`Grid::widget`] each frame, then pass focused keyboard events and mouse
//! events to [`Grid::handle_event`]. Rendering and hit-testing share one layout.
//!
//! The application owns terminal setup, component focus, fetching, persistence,
//! and animation timing. Owned datasets sort globally; external grids request
//! one source-sorted page at a time.
//!
//! ```
//! use ratagrid::{Column, Grid};
//! let columns = vec![Column::new("Value", 12, |n: &u64| n.to_string())
//!     .sortable(|a, b| a.cmp(b))];
//! let mut grid = Grid::new(columns, vec![100, 2, 10]);
//! // In your draw callback: frame.render_widget(grid.widget(), area);
//! // In your focused event loop: grid.handle_event(&event);
//! ```
//!
//! See the bundled [feature guide](guides::features),
//! [pagination guide](guides::pagination), and [updates guide](guides::updates).

/// Integration guides bundled with the crate, independent of repository access.
pub mod guides {
    #[doc = include_str!("../docs/rustdoc/features.md")]
    pub mod features {}
    #[doc = include_str!("../docs/rustdoc/pagination.md")]
    pub mod pagination {}
    #[doc = include_str!("../docs/rustdoc/updates.md")]
    pub mod updates {}
}

mod columns;
mod details;
pub use details::{CellDetail, CellDetailsOptions};
mod features;
mod input;
pub use columns::ColumnOrderError;
pub use features::{CopyTarget, ExternalFilterError, LoadState};
mod model;
mod pagination;
mod render;
mod text;
pub use model::{Column, GridModel, Sort, SortDirection};
pub use pagination::{CursorPageRequest, PageError, PageRequest, PageState, PaginationMode};
use pagination::{Navigation, Pagination};
use std::{collections::HashMap, num::NonZeroUsize, ops::Range, time::Duration};

use ratatui_core::{
    layout::Rect,
    style::{Color, Modifier, Style},
};

/// Changes emitted by grid interaction. Row indices refer to insertion order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    SortChanged(Option<Sort>),
    SelectionChanged(usize),
    RowActivated(usize),
    /// The active cell moved within a row. Row is an insertion index.
    CursorMoved {
        row: usize,
        column: usize,
    },
    ColumnResized {
        column: usize,
        width: u16,
    },
    Scrolled,
    HoverChanged,
    /// The optional full-value panel opened, closed, or scrolled.
    CellDetailsChanged,
    HeaderFocused(usize),
    HeaderBlurred,
    PageChanged(PageState),
    PageRequested(PageRequest),
    /// Fetch using this opaque token, preserving the request with its response.
    CursorPageRequested(CursorPageRequest),
    /// The query/filter changed. Read the current query using `Grid::search_query`.
    FilterChanged,
    /// Redraw the search entry after typing, starting, or cancelling a search.
    SearchEdited,
    /// The explicit bulk selection changed; read `GridModel::selected_indices`.
    RowsSelected,
    /// The application owns clipboard access. Obtain text using `Grid::copy_text`.
    CopyRequested(CopyTarget),
}

/// Colors for the grid. Replace these to match your application's theme.
#[derive(Debug, Clone, Copy)]
pub struct GridStyle {
    pub cell: Style,
    pub header: Style,
    pub hover: Style,
    pub selected: Style,
    pub focused_header: Style,
    pub separator: Style,
    /// Style patched onto the active cell after selection and animation styles.
    pub cursor: Style,
    /// Style for explicitly marked rows, composed before active row/cursor styling.
    pub marked: Style,
    /// Loading, empty, and search messages.
    pub status: Style,
    /// Source error messages.
    pub error: Style,
    /// Temporary style for cells marked with [`Grid::flash_cell`].
    pub flash: Style,
}

impl Default for GridStyle {
    fn default() -> Self {
        Self {
            cell: Style::default(),
            header: Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            hover: Style::default().bg(Color::DarkGray),
            selected: Style::default().bg(Color::Blue).fg(Color::White),
            focused_header: Style::default().bg(Color::DarkGray).fg(Color::Cyan),
            separator: Style::default().fg(Color::DarkGray),
            marked: Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
            status: Style::default().fg(Color::DarkGray),
            error: Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            cursor: Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD),
            flash: Style::default()
                .fg(Color::Rgb(255, 235, 188))
                .bg(Color::Rgb(125, 78, 24))
                .add_modifier(Modifier::BOLD),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Hit {
    Header(usize),
    Row(usize),
    Resize(usize),
    Page(Navigation),
    Retry,
}

#[derive(Debug, Clone, Copy)]
struct ColumnLayout {
    column: usize,
    start: i64,
    end: i64,
    clip_start: u16,
    clip_end: u16,
}

#[derive(Debug, Clone, Copy)]
struct Drag {
    column: usize,
    x: u16,
    width: u16,
}

struct CellFlash {
    elapsed: Duration,
    duration: Duration,
}

/// Interactive grid state plus its framework-independent model.
pub struct Grid<T> {
    model: GridModel<T>,
    details: details::DetailsState,
    style: GridStyle,
    area: Rect,
    layout: Vec<ColumnLayout>,
    row_offset: usize,
    column_offset: usize,
    hover: Option<Hit>,
    header_focus: Option<usize>,
    cursor_column: Option<usize>,
    drag: Option<Drag>,
    pagination: Option<Pagination>,
    page_buttons: Vec<(Rect, Navigation, bool)>,
    flashes: HashMap<(usize, usize), CellFlash>,
    column_order: Vec<usize>,
    column_visible: Vec<bool>,
    column_pinned: Vec<bool>,
    search_query: String,
    search_draft: Option<String>,
    page_error: Option<String>,
    retry_button: Option<Rect>,
    selection_anchor: Option<usize>,
    range_base: Option<std::collections::BTreeSet<usize>>,
}

impl<T> Grid<T> {
    pub fn new(columns: Vec<Column<T>>, rows: Vec<T>) -> Self {
        let column_count = columns.len();
        Self {
            model: GridModel::new(columns, rows),
            details: details::DetailsState::default(),
            style: GridStyle::default(),
            area: Rect::default(),
            layout: vec![],
            row_offset: 0,
            column_offset: 0,
            hover: None,
            header_focus: None,
            cursor_column: None,
            drag: None,
            pagination: None,
            page_buttons: vec![],
            flashes: HashMap::new(),
            column_order: (0..column_count).collect(),
            column_visible: vec![true; column_count],
            column_pinned: vec![false; column_count],
            search_query: String::new(),
            search_draft: None,
            page_error: None,
            retry_button: None,
            selection_anchor: None,
            range_base: None,
        }
    }

    /// Preserve selection across owned replacements and external page reloads.
    /// The callback returns a stable, unique application ID (for example a database
    /// primary key). A missing or duplicated selected ID clears selection.
    /// While an external request is loading, selection is hidden; an accepted
    /// response restores it only if that page contains the ID exactly once.
    /// Restored selection is revealed vertically if it belongs to the current page.
    /// Action indices still refer to the current resident rows, not IDs.
    ///
    /// ```
    /// use ratagrid::{Column, Grid};
    /// let mut grid = Grid::new(vec![Column::new("ID", 8, |id: &u64| id.to_string())], vec![1, 2])
    ///     .with_row_id(|id| *id);
    /// grid.replace_rows(vec![2, 1]);
    /// ```
    pub fn with_row_id<K: Eq + 'static>(self, row_id: impl Fn(&T) -> K + 'static) -> Self {
        Self {
            model: self.model.with_row_id(row_id),
            ..self
        }
    }

    pub fn model(&self) -> &GridModel<T> {
        &self.model
    }
    pub fn style_mut(&mut self) -> &mut GridStyle {
        &mut self.style
    }
    pub fn row_offset(&self) -> usize {
        self.row_offset
    }
    pub fn column_offset(&self) -> usize {
        self.column_offset
    }
    /// Active cell as (insertion row index, column index), when the selected
    /// record belongs to the current page and headers do not have focus.
    pub fn cursor(&self) -> Option<(usize, usize)> {
        if self.header_focus.is_some() {
            return None;
        }
        let position = self.model.selected_position()?;
        if !self.row_range().contains(&position) {
            return None;
        }
        let column = self.cursor_column?;
        self.is_column_visible(column)
            .then_some((self.model.selected_index()?, column))
    }

    pub fn column_width(&self, column: usize) -> Option<u16> {
        self.model.columns.get(column).map(|c| c.width)
    }

    /// Select a resident row by insertion index (the index into `model().rows()`).
    /// Reveals its sorted/filtered position, switching client pages if necessary.
    /// External indices refer only to the loaded page; this never fetches a row.
    /// Invalid or filtered-out indices leave state unchanged. Bulk marks are unchanged.
    /// Render afterward to use the current viewport and refresh mouse geometry.
    pub fn select_row(&mut self, index: usize) -> Option<Action> {
        if index >= self.model.rows().len() {
            return None;
        }
        let position = (0..self.model.visible_len())
            .find(|&position| self.model.index_at(position) == Some(index))?;
        let had_details = self.cell_detail().is_some();
        self.clear_cell_details();
        let mut page_changed = false;
        if let Some(p) = self.pagination.as_mut()
            && p.mode == PaginationMode::Client
        {
            let page = position / p.size.get();
            page_changed = p.page != page;
            p.page = page;
        }
        self.select_position(position)
            .or_else(|| {
                page_changed.then(|| Action::PageChanged(self.page_state().expect("client page")))
            })
            .or_else(|| had_details.then_some(Action::CellDetailsChanged))
    }

    /// Edit a resident row by insertion index, preserving selection. Owned rows
    /// are repositioned in the active global sort; external pages retain source order.
    /// This changes local data only. Persist edits through your application's source.
    ///
    /// ```
    /// use ratagrid::{Column, Grid};
    /// use std::time::Duration;
    /// let mut grid = Grid::new(vec![Column::new("Value", 12, |n: &u64| n.to_string())], vec![100]);
    /// assert!(grid.update_row(0, |value| *value += 50));
    /// grid.flash_cell(0, 0, Duration::from_millis(900));
    /// assert!(grid.advance_animations(Duration::from_millis(450)));
    /// assert!(!grid.advance_animations(Duration::from_millis(450)));
    /// assert_eq!(grid.model().rows(), &[150]);
    /// ```
    pub fn update_row(&mut self, index: usize, update: impl FnOnce(&mut T)) -> bool {
        let updated = if self.is_external() {
            self.model.update_row_in_place(index, update)
        } else {
            self.model.update_row(index, update)
        };
        if updated {
            self.hover = None;
            self.clear_cell_details();
        }
        updated
    }

    /// Highlight a cell, following its record across sorting and client pages.
    /// Advance its fade through [`Self::advance_animations`] in your event loop.
    /// Returns false for invalid row/column indices or a zero duration.
    pub fn flash_cell(&mut self, index: usize, column: usize, duration: Duration) -> bool {
        if index >= self.model.rows().len()
            || column >= self.model.columns.len()
            || duration.is_zero()
        {
            return false;
        }
        self.flashes.insert(
            (index, column),
            CellFlash {
                elapsed: Duration::ZERO,
                duration,
            },
        );
        true
    }

    /// Advance cell fades by your application's elapsed frame time. Returns
    /// whether more animation frames are needed. No clock or timer is owned by the grid.
    pub fn advance_animations(&mut self, elapsed: Duration) -> bool {
        self.flashes.retain(|_, flash| {
            flash.elapsed = flash.elapsed.saturating_add(elapsed);
            flash.elapsed < flash.duration
        });
        self.is_animating()
    }

    pub fn is_animating(&self) -> bool {
        !self.flashes.is_empty()
    }

    pub fn clear_animations(&mut self) {
        self.flashes.clear();
    }

    /// Replace an owned dataset, preserving sorting and keyed selection.
    /// Without [`Self::with_row_id`], selection is cleared.
    ///
    /// # Panics
    /// Panics in external pagination mode; use [`Self::set_page_data`] instead.
    pub fn replace_rows(&mut self, rows: Vec<T>) {
        assert!(
            !self.is_external(),
            "Use set_page_data for an externally paginated grid"
        );
        self.model.replace_rows(rows);
        self.selection_anchor = self.model.selected_index();
        self.range_base = None;
        self.clear_animations();
        self.clamp_page();
        self.row_offset = 0;
        self.reveal_selected_row();
        self.hover = None;
        self.clear_cell_details();
    }

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

    fn validate_cursor_request(&self, request: &CursorPageRequest) -> Result<(), PageError> {
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

    fn is_external(&self) -> bool {
        self.pagination
            .as_ref()
            .is_some_and(|p| p.mode != PaginationMode::Client)
    }
    fn page_action(&mut self) -> Option<Action> {
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
    fn clamp_page(&mut self) {
        if let Some(state) = self.page_state()
            && let Some(count) = state.page_count
        {
            self.pagination.as_mut().expect("pagination").page = state.page.min(count - 1);
        }
    }
    fn row_range(&self) -> Range<usize> {
        let len = self.model.visible_len();
        match self.page_state() {
            Some(p) if p.mode == PaginationMode::Client => {
                let start = p.page.saturating_mul(p.page_size.get()).min(len);
                start..start.saturating_add(p.page_size.get()).min(len)
            }
            _ => 0..len,
        }
    }
    fn navigate(&mut self, navigation: Navigation) -> Option<Action> {
        let state = self.page_state()?;
        let page = match navigation {
            Navigation::First => 0,
            Navigation::Previous => state.page.saturating_sub(1),
            Navigation::Next => state.page.saturating_add(1),
            Navigation::Last => state.page_count? - 1,
        };
        self.set_page(page)
    }
    fn toggle_sort(&mut self, column: usize) -> Option<Action> {
        if self.is_external() {
            self.range_base = None;
            if !self.model.request_sort(column) {
                return None;
            }
            let p = self.pagination.as_mut()?;
            p.page = 0;
            p.reset_cursors();
            self.page_action()
        } else {
            self.range_base = None;
            if !self.model.toggle_sort(column) {
                return None;
            }
            if let Some(p) = self.pagination.as_mut() {
                p.page = 0;
                self.row_offset = 0;
            }
            Some(Action::SortChanged(self.model.sort()))
        }
    }

    pub fn widget(&mut self) -> GridWidget<'_, T> {
        GridWidget(self)
    }
}

/// A widget borrowing the grid, updating its hit-test geometry while rendering.
pub struct GridWidget<'a, T>(&'a mut Grid<T>);

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
mod paging;
mod render;
mod text;
pub use model::{Column, GridModel, Sort, SortDirection};
pub use pagination::{CursorPageRequest, PageError, PageRequest, PageState, PaginationMode};
use pagination::{Navigation, Pagination};
use std::{collections::HashMap, time::Duration};

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

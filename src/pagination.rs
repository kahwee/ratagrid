use crate::Sort;
use std::{
    fmt,
    num::NonZeroUsize,
    sync::atomic::{AtomicU64, Ordering},
};

// Request IDs must not restart when an app replaces a grid or has several grids.
static NEXT_REQUEST: AtomicU64 = AtomicU64::new(1);
pub(crate) fn next_revision() -> u64 {
    let mut id = NEXT_REQUEST.load(Ordering::Relaxed);
    loop {
        let next = id.checked_add(1).expect("page request IDs exhausted");
        match NEXT_REQUEST.compare_exchange_weak(id, next, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => return id,
            Err(current) => id = current,
        }
    }
}

/// Who owns the full dataset and performs sorting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaginationMode {
    /// All records are resident; Ratagrid sorts the whole dataset before paging it.
    Client,
    /// Only the returned page is resident; the application fetches and sorts globally.
    External,
    /// Opaque continuation tokens; the application performs keyset queries.
    Cursor,
}

/// A page/sort request. Keep this value with an asynchronous request and return it
/// unchanged to [`crate::Grid::set_page_data`] to reject stale responses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageRequest {
    /// Zero-based page number.
    pub page: usize,
    pub page_size: NonZeroUsize,
    pub sort: Option<Sort>,
    /// Opaque process-wide request ID. Changes for each new grid/request;
    /// do not assume consecutive values or persist it across processes.
    pub revision: u64,
}

impl PageRequest {
    pub fn offset(self) -> usize {
        self.page.saturating_mul(self.page_size.get())
    }
}

/// A cursor request. Capture this value and the search query before dispatching
/// asynchronous work; return the original request with its response.
///
/// `cursor: None` starts from the beginning. Tokens are opaque UTF-8 strings;
/// serialize structured keys or encode binary tokens in the application.
/// `page` is a navigation ordinal, never a database offset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CursorPageRequest {
    pub page: usize,
    pub page_size: NonZeroUsize,
    pub sort: Option<Sort>,
    /// Process-wide request ID with the same lifecycle as [`PageRequest::revision`].
    pub revision: u64,
    pub cursor: Option<String>,
}

/// Read-only pagination metadata. Empty datasets have one empty page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageState {
    pub mode: PaginationMode,
    pub page: usize,
    pub page_size: NonZeroUsize,
    /// None in cursor mode or when an offset source has not supplied a total.
    pub page_count: Option<usize>,
    /// None in cursor mode or when an offset source has not supplied a total.
    pub total_rows: Option<usize>,
    /// Whether navigation can request the next page. Cursor mode uses the source
    /// continuation token; cursor and unknown-total sources disable Next while loading.
    pub has_next_page: bool,
    /// Records resident in memory: the full dataset in client mode, one page in external mode.
    pub loaded_rows: usize,
    pub loading: bool,
}

/// A page response was not accepted. Failed responses leave the current grid unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageError {
    /// An offset response was supplied to a client or cursor grid.
    NotExternal,
    /// A cursor response was supplied to a client or offset grid.
    NotCursor,
    StaleResponse,
    WrongRowCount {
        expected: usize,
        actual: usize,
    },
}
impl fmt::Display for PageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotExternal => f.write_str("grid is not offset externally paginated"),
            Self::NotCursor => f.write_str("grid is not cursor paginated"),
            Self::StaleResponse => {
                f.write_str("page response is obsolete, foreign, or already applied")
            }
            Self::WrongRowCount { expected, actual } => write!(
                f,
                "expected {expected} rows for this page, received {actual}"
            ),
        }
    }
}
impl std::error::Error for PageError {}

pub(crate) struct Pagination {
    pub mode: PaginationMode,
    pub page: usize,
    pub size: NonZeroUsize,
    pub total: Option<usize>,
    pub has_next: bool,
    pub revision: u64,
    pub loading: bool,
    // The request boundary for each visited page, plus the available next page.
    pub cursors: Vec<Option<String>>,
}
impl Pagination {
    pub fn reset_cursors(&mut self) {
        if self.mode == PaginationMode::Cursor {
            self.cursors.clear();
            self.cursors.push(None);
            self.has_next = false;
        }
    }

    pub fn count(&self, total: usize) -> usize {
        total.div_ceil(self.size.get()).max(1)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Navigation {
    First,
    Previous,
    Next,
    Last,
}

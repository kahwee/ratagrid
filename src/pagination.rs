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

/// Read-only pagination metadata. Empty datasets have one empty page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageState {
    pub mode: PaginationMode,
    pub page: usize,
    pub page_size: NonZeroUsize,
    /// None when the external source has not supplied a total.
    pub page_count: Option<usize>,
    /// None when the external source has not supplied a total.
    pub total_rows: Option<usize>,
    /// Whether navigation can request the next page. False while an unknown-total page loads.
    pub has_next_page: bool,
    /// Records resident in memory: the full dataset in client mode, one page in external mode.
    pub loaded_rows: usize,
    pub loading: bool,
}

/// A page response was not accepted. Failed responses leave the current grid unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageError {
    NotExternal,
    StaleResponse,
    WrongRowCount { expected: usize, actual: usize },
}
impl fmt::Display for PageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotExternal => f.write_str("grid is not externally paginated"),
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
}
impl Pagination {
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

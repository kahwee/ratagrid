use ratatui_core::style::Style;
use std::{
    any::Any,
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};

type RowKey<T> = Box<dyn Fn(&T) -> Box<dyn Any>>;
type KeyMatches<T> = Box<dyn Fn(&T, &dyn Any) -> bool>;

struct RowIdentity<T> {
    key: RowKey<T>,
    matches: KeyMatches<T>,
}

type Formatter<T> = Box<dyn Fn(&T) -> String>;
type CellStyle<T> = Box<dyn Fn(&T) -> Style>;
type BarValue<T> = Box<dyn Fn(&T) -> f64>;

pub(crate) struct BarChart<T> {
    pub(crate) max: f64,
    pub(crate) value: BarValue<T>,
}
type RowFilter<T> = Box<dyn Fn(&T) -> bool>;

type Comparator<T> = Box<dyn Fn(&T, &T) -> Ordering>;

/// A typed column. Sorting compares records, independently of their display text.
pub struct Column<T> {
    pub(crate) title: String,
    pub(crate) width: u16,
    pub(crate) format: Formatter<T>,
    pub(crate) compare: Option<Comparator<T>>,
    pub(crate) external_sortable: bool,
    pub(crate) cell_style: Option<CellStyle<T>>,
    pub(crate) bar_chart: Option<BarChart<T>>,
}

impl<T> Column<T> {
    /// Create a column with a width in terminal cells (minimum four).
    pub fn new(
        title: impl Into<String>,
        width: u16,
        format: impl Fn(&T) -> String + 'static,
    ) -> Self {
        Self {
            title: title.into(),
            width: width.max(4),
            format: Box::new(format),
            compare: None,
            external_sortable: false,
            cell_style: None,
            bar_chart: None,
        }
    }

    /// Render a horizontal bar instead of the formatted text in each data cell.
    /// `max` is the shared scale: that value fills the column, excluding its
    /// separator. Bars resize with the column and use its normal cell style.
    /// Use [`Self::cell_style`] to color bars by record.
    ///
    /// Values are clamped to `0..=max`. Non-finite values and a non-positive or
    /// non-finite maximum render empty. Eighth-cell blocks represent fractions;
    /// values smaller than one eighth of a cell render empty.
    /// The application chooses the maximum, so filtering and paging do not
    /// silently rescale bars. The formatter remains the text used for search,
    /// copying and full-value details; sorting still uses the comparator.
    ///
    /// ```
    /// use ratagrid::Column;
    /// let column = Column::new("Usage", 25, |gb: &f64| format!("{gb:.1} GB"))
    ///     .bar_chart(60.0, |gb| *gb)
    ///     .sortable(|a, b| a.total_cmp(b));
    /// ```
    pub fn bar_chart(mut self, max: f64, value: impl Fn(&T) -> f64 + 'static) -> Self {
        self.bar_chart = Some(BarChart {
            max,
            value: Box::new(value),
        });
        self
    }

    /// Style data cells using the underlying record, independently of display text.
    /// This patches the base cell style; selection, hover, flashes and the cell cursor apply afterward.
    /// The callback runs only for visible cells and should remain inexpensive.
    pub fn cell_style(mut self, style: impl Fn(&T) -> Style + 'static) -> Self {
        self.cell_style = Some(Box::new(style));
        self
    }

    /// Enable sorting with a comparator over the underlying records.
    pub fn sortable(mut self, compare: impl Fn(&T, &T) -> Ordering + 'static) -> Self {
        self.compare = Some(Box::new(compare));
        self
    }

    /// Enable source-side sorting in an externally paginated grid, without an
    /// unused local comparator. Owned grids treat this column as unsortable.
    pub fn sortable_external(mut self) -> Self {
        self.external_sortable = true;
        self
    }
}

/// The direction of an active sort.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

/// Single-column sort state. `None` restores insertion order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sort {
    pub column: usize,
    pub direction: SortDirection,
}

/// Framework-independent ordering and selection over owned records.
///
/// Selection refers to an insertion index, so sorting never selects a different record.
pub struct GridModel<T> {
    pub(crate) columns: Vec<Column<T>>,
    rows: Vec<T>,
    pub(crate) order: Vec<usize>,
    selected: Option<usize>,
    selected_position: Option<usize>,
    sort: Option<Sort>,
    identity: Option<RowIdentity<T>>,
    selected_id: Option<Box<dyn Any>>,
    marked: BTreeSet<usize>,
    marked_ids: BTreeMap<usize, Box<dyn Any>>,
    filter: Option<RowFilter<T>>,
    search: String,
}

impl<T> GridModel<T> {
    pub fn new(columns: Vec<Column<T>>, rows: Vec<T>) -> Self {
        let order = (0..rows.len()).collect();
        Self {
            columns,
            rows,
            order,
            selected: None,
            selected_position: None,
            sort: None,
            identity: None,
            selected_id: None,
            marked: BTreeSet::new(),
            marked_ids: BTreeMap::new(),
            filter: None,
            search: String::new(),
        }
    }

    /// Preserve selection across replacements using an application-owned row ID.
    /// IDs must be stable and unique. A missing or duplicated selected ID clears
    /// selection. Without this callback, replacements clear selection as before.
    /// Only active/marked IDs are retained; no per-row ID index is allocated.
    pub fn with_row_id<K: Eq + 'static>(mut self, row_id: impl Fn(&T) -> K + 'static) -> Self {
        let row_id = Rc::new(row_id);
        let extract = Rc::clone(&row_id);
        self.identity = Some(RowIdentity {
            key: Box::new(move |row| Box::new(extract(row))),
            matches: Box::new(move |row, key| {
                key.downcast_ref::<K>()
                    .is_some_and(|key| row_id(row) == *key)
            }),
        });
        self.refresh_selected_id();
        self.refresh_marked_ids();
        self
    }

    fn refresh_selected_id(&mut self) {
        self.selected_id = self.selected.and_then(|index| {
            self.identity
                .as_ref()
                .map(|identity| (identity.key)(&self.rows[index]))
        });
    }

    fn restore_selection(&mut self) {
        self.selected = None;
        self.selected_position = None;
        if let (Some(identity), Some(key)) = (&self.identity, &self.selected_id) {
            let mut matches = self
                .rows
                .iter()
                .enumerate()
                .filter(|(_, row)| (identity.matches)(row, key.as_ref()));
            if let Some((index, _)) = matches.next()
                && matches.next().is_none()
            {
                self.selected = Some(index);
            }
        }
        if self.selected.is_none() {
            self.selected_id = None;
        }
        self.marked.clear();
        if let Some(identity) = &self.identity {
            for key in self.marked_ids.values() {
                let mut matches = self
                    .rows
                    .iter()
                    .enumerate()
                    .filter(|(_, row)| (identity.matches)(row, key.as_ref()));
                if let Some((index, _)) = matches.next()
                    && matches.next().is_none()
                {
                    self.marked.insert(index);
                }
            }
        }
        self.refresh_marked_ids();
    }

    fn refresh_marked_ids(&mut self) {
        self.marked_ids.clear();
        if let Some(identity) = &self.identity {
            for &index in &self.marked {
                self.marked_ids
                    .insert(index, (identity.key)(&self.rows[index]));
            }
        }
    }

    /// Resident insertion indices explicitly marked for bulk actions, in insertion order.
    /// The active cursor row is independent of this set.
    pub fn selected_indices(&self) -> impl Iterator<Item = usize> + '_ {
        self.marked.iter().copied()
    }

    pub fn is_row_selected(&self, index: usize) -> bool {
        self.marked.contains(&index)
    }

    /// Replace the bulk selection. Invalid indices are ignored.
    pub fn set_selected_indices(&mut self, indices: impl IntoIterator<Item = usize>) -> bool {
        let marked = indices
            .into_iter()
            .filter(|&i| i < self.rows.len())
            .collect();
        if self.marked == marked {
            return false;
        }
        self.marked = marked;
        self.refresh_marked_ids();
        true
    }

    pub fn toggle_row_selection(&mut self, index: usize) -> bool {
        if index >= self.rows.len() {
            return false;
        }
        if !self.marked.remove(&index) {
            self.marked.insert(index);
        }
        if let Some(identity) = &self.identity {
            if self.marked.contains(&index) {
                self.marked_ids
                    .insert(index, (identity.key)(&self.rows[index]));
            } else {
                self.marked_ids.remove(&index);
            }
        }
        true
    }

    pub fn clear_row_selection(&mut self) -> bool {
        let changed = !self.marked.is_empty();
        self.marked.clear();
        self.marked_ids.clear();
        changed
    }

    /// Number of rows after filtering, independently of resident storage size.
    pub fn visible_len(&self) -> usize {
        self.order.len()
    }

    /// Filter owned rows using actual record values. Search and this predicate combine.
    pub fn set_filter(&mut self, filter: impl Fn(&T) -> bool + 'static) {
        self.filter = Some(Box::new(filter));
        self.apply_sort();
    }

    pub fn clear_filter(&mut self) {
        self.filter = None;
        self.apply_sort();
    }

    /// Case-insensitive substring search across all defined formatted columns,
    /// removing terminal and bidirectional formatting controls as rendering
    /// and copying do.
    /// Formatting is evaluated for all resident rows when the query changes.
    pub fn set_search(&mut self, query: impl Into<String>) {
        self.search = query
            .into()
            .chars()
            .filter(|c| !crate::text::is_unsafe(*c))
            .collect::<String>()
            .to_lowercase();
        self.apply_sort();
    }

    fn matches_filter(&self, row: &T) -> bool {
        self.filter.as_ref().is_none_or(|filter| filter(row))
            && (self.search.is_empty()
                || self.columns.iter().any(|column| {
                    (column.format)(row)
                        .chars()
                        .filter(|c| !crate::text::is_unsafe(*c))
                        .collect::<String>()
                        .to_lowercase()
                        .contains(&self.search)
                }))
    }

    /// Discard resident records while retaining a keyed selection during a fetch.
    pub(crate) fn clear_rows_for_request(&mut self) {
        self.rows.clear();
        self.order.clear();
        self.selected = None;
        self.selected_position = None;
        self.marked.clear();
    }

    pub fn rows(&self) -> &[T] {
        &self.rows
    }
    pub fn sort(&self) -> Option<Sort> {
        self.sort
    }
    pub fn selected(&self) -> Option<&T> {
        self.selected.map(|i| &self.rows[i])
    }
    pub fn selected_index(&self) -> Option<usize> {
        self.selected
    }
    pub fn selected_position(&self) -> Option<usize> {
        self.selected_position
    }
    pub fn row_at(&self, position: usize) -> Option<&T> {
        self.index_at(position).map(|i| &self.rows[i])
    }
    /// Resolve a sorted position to the resident record's insertion index.
    pub fn index_at(&self, position: usize) -> Option<usize> {
        self.order.get(position).copied()
    }

    /// Edit one resident record, maintaining stable sorting and selection.
    /// Returns false for an invalid index without calling `update`.
    /// Only the edited row is repositioned: logarithmic comparisons, with up to
    /// linear index lookup/movement. Small moves shift only the crossed span
    /// of ordering indices; the full dataset is never re-sorted.
    pub fn update_row(&mut self, index: usize, update: impl FnOnce(&mut T)) -> bool {
        if !self.update_row_in_place(index, update) {
            return false;
        }
        if self.filter.is_some() || !self.search.is_empty() {
            self.apply_sort();
            return true;
        }
        let Some(sort) = self.sort else {
            return true;
        };
        let old = if self.selected == Some(index) {
            self.selected_position.expect("selected position")
        } else {
            order_position(&self.order, index).expect("resident row")
        };
        let compare = self.columns[sort.column]
            .compare
            .as_ref()
            .expect("sortable column");
        let ordered = |a: usize, b: usize| {
            let result = compare(&self.rows[a], &self.rows[b]);
            let result = match sort.direction {
                SortDirection::Ascending => result,
                SortDirection::Descending => result.reverse(),
            };
            result.then(a.cmp(&b))
        };
        if (old == 0 || ordered(self.order[old - 1], index) != Ordering::Greater)
            && (old + 1 == self.order.len()
                || ordered(index, self.order[old + 1]) != Ordering::Greater)
        {
            return true;
        }
        let new = move_order_index(&mut self.order, old, index, ordered);
        self.selected_position = self.selected_position.map(|position| {
            if self.selected == Some(index) {
                new
            } else if old < position && new >= position {
                position - 1
            } else if new <= position && old > position {
                position + 1
            } else {
                position
            }
        });
        true
    }

    pub(crate) fn update_row_in_place(
        &mut self,
        index: usize,
        update: impl FnOnce(&mut T),
    ) -> bool {
        let Some(row) = self.rows.get_mut(index) else {
            return false;
        };
        update(row);
        if self.selected == Some(index) {
            self.refresh_selected_id();
        }
        if self.marked.contains(&index)
            && let Some(identity) = &self.identity
        {
            self.marked_ids
                .insert(index, (identity.key)(&self.rows[index]));
        }
        true
    }
    pub fn select(&mut self, position: usize) -> bool {
        let Some(&index) = self.order.get(position) else {
            return false;
        };
        let changed = self.selected != Some(index);
        self.selected = Some(index);
        self.selected_position = Some(position);
        self.refresh_selected_id();
        changed
    }

    /// Cycle ascending → descending → insertion order. Returns false for unsortable columns.
    pub fn toggle_sort(&mut self, column: usize) -> bool {
        if self
            .columns
            .get(column)
            .and_then(|c| c.compare.as_ref())
            .is_none()
        {
            return false;
        }
        if !self.request_sort(column) {
            return false;
        }
        self.apply_sort();
        true
    }

    /// Update sort metadata without sorting a server-provided page locally.
    pub(crate) fn request_sort(&mut self, column: usize) -> bool {
        if !self
            .columns
            .get(column)
            .is_some_and(|c| c.compare.is_some() || c.external_sortable)
        {
            return false;
        }
        self.sort = match self.sort {
            Some(Sort {
                column: c,
                direction: SortDirection::Ascending,
            }) if c == column => Some(Sort {
                column,
                direction: SortDirection::Descending,
            }),
            Some(Sort {
                column: c,
                direction: SortDirection::Descending,
            }) if c == column => None,
            _ => Some(Sort {
                column,
                direction: SortDirection::Ascending,
            }),
        };
        true
    }

    /// Replace records, preserving the active sort and selection when a unique
    /// matching ID is configured with [`Self::with_row_id`].
    pub fn replace_rows(&mut self, rows: Vec<T>) {
        self.rows = rows;
        self.restore_selection();
        self.apply_sort();
    }

    pub(crate) fn replace_rows_in_order(&mut self, rows: Vec<T>) {
        self.rows = rows;
        self.restore_selection();
        self.order = (0..self.rows.len()).collect();
        self.selected_position = self.selected;
    }

    fn apply_sort(&mut self) {
        self.order = (0..self.rows.len())
            .filter(|&index| self.matches_filter(&self.rows[index]))
            .collect();
        if let Some(sort) = self.sort {
            let compare = self.columns[sort.column]
                .compare
                .as_ref()
                .expect("sortable column");
            let rows = &self.rows;
            self.order.sort_by(|&a, &b| {
                let result = compare(&rows[a], &rows[b]);
                match sort.direction {
                    SortDirection::Ascending => result,
                    SortDirection::Descending => result.reverse(),
                }
            });
        }
        self.selected_position = self
            .selected
            .and_then(|i| self.order.iter().position(|&r| r == i));
    }
}

// Keep movement out of the hot update function so the unchanged-key path is
// less sensitive to inlining and code layout of the movement implementation.
#[inline(never)]
fn move_order_index(
    order: &mut Vec<usize>,
    old: usize,
    index: usize,
    ordered: impl Fn(usize, usize) -> Ordering,
) -> usize {
    // Small moves copy only the crossed span. Large moves retain Vec's path,
    // which measured better than general rotation on Windows.
    let new = if old > 0 && ordered(order[old - 1], index) == Ordering::Greater {
        order[..old].partition_point(|&i| ordered(i, index) == Ordering::Less)
    } else {
        old + order[old + 1..].partition_point(|&i| ordered(i, index) == Ordering::Less)
    };
    if new.abs_diff(old) > order.len() / 4 {
        order.remove(old);
        order.insert(new, index);
    } else if new < old {
        order.copy_within(new..old, new + 1);
        order[new] = index;
    } else {
        order.copy_within(old + 1..new + 1, old);
        order[new] = index;
    }
    new
}

// Compile the linear scan independently of the generic updater and movement
// branches. Its code layout must not grow with either implementation.
#[inline(never)]
fn order_position(order: &[usize], index: usize) -> Option<usize> {
    order.iter().position(|&i| i == index)
}

# Updates and animation

A click anywhere in your application can change a grid record. Ratagrid emits input actions; your event loop decides what they change. The playground's **Boost** button sits below the table and updates three cells in the selected row, or the first visible row if no record on this page is selected.

```sh
cargo run --release --locked --example playground
```

Select a row, then click **Boost** or press **B**. Latency, Delta and Memory count up over 650 ms, with a progress indicator and an amber highlight that fades over 1.1 seconds. **A** toggles motion; turning it off completes pending updates immediately and clears their highlights. Repeated clicks restart from the current values. You can animate several rows concurrently. **L** also highlights cells after live feed updates.


## Change a row from another control

```rust
use ratagrid::{Column, Grid};
use std::time::Duration;

struct Job { latency_ms: u64 }
let columns = vec![Column::new("Latency", 14, |job: &Job| format!("{} ms", job.latency_ms))
    .sortable(|a, b| a.latency_ms.cmp(&b.latency_ms))];
let mut grid = Grid::new(columns, vec![Job { latency_ms: 100 }]);

// In your button-click handler. The index refers to model().rows().
let index = grid.model().selected_index().unwrap_or(0);
if grid.update_row(index, |job| job.latency_ms += 50) {
    grid.flash_cell(index, 0, Duration::from_millis(900));
}
```

`update_row` changes typed data, so formatting on the next render shows the new values. It returns false for an invalid row without calling the update closure. `GridModel::index_at(position)` maps a sorted position to a resident row index.

Owned grids preserve selection, maintain the active global sort and retain stable ties in both directions. They reposition only the edited row, using logarithmic comparator calls and up to linear index lookup/movement. Updating an unsorted, unfiltered row requires no ordering scan. A changed sort key can move a row out of the current viewport or client page; navigation still follows the selected record.

With search or filtering active, only the edited record is re-evaluated. Rows can enter or leave the results without filtering or sorting the entire dataset again. Selection remains attached to its resident record, even while that record is hidden. Filters, formatters, and comparators should depend on record data. If a callback depends on external state that changes, reapply the filter or model search to rebuild all results.

External grids update only the current resident page, preserving the source's order and skipping local comparators. The edit is local: persist it through your API/database, then reload the page if server ordering or totals have changed. External navigation clears animations. With a row ID callback, selection is restored if the next accepted page contains the selected ID exactly once; otherwise selection clears. Application value tweens must also be cancelled when a page is replaced; the playground demonstrates this.

## Apply a live-feed batch

`Grid::update_rows` and `GridModel::update_rows` accept resident insertion indices
and an `FnMut(index, &mut record)` callback. Valid indices are visited in iterator
order, duplicates are edited repeatedly, and invalid indices are skipped. The
return value counts valid edits, including repeats. An empty or invalid-only batch
does not rebuild ordering or change interaction state.

```rust
use ratagrid::{Column, Grid};
let mut grid = Grid::new(vec![Column::new("Value", 10, |n: &i64| n.to_string())], vec![10, 20, 30]);
assert_eq!(grid.update_rows([0, 2], |_, n| *n += 5), 2);
assert_eq!(grid.model().rows(), &[15, 20, 35]);
```

Owned batches rebuild filtering and stable sorting once, then refresh client page
state once. External batches preserve source order and never invoke local
comparators. Keyed active and marked selections track edits to their IDs. This
is not a transaction: a callback panic does not roll back previous edits.
Use single-row updates for small changes; a batch does one full filter/sort pass
and is useful when repeated ordering movements would cost more. The
[`live_updates` benchmark](https://github.com/kahwee/ratagrid/blob/main/examples/live_updates.rs) measures both strategies
as well as identity restoration and Unicode search.

## Append and expire owned records

**Added in 0.5.0.** `append_rows(records)` adds records without replacing the
dataset. It filters only incoming rows, sorts their visible indices and merges
them into existing ordering. Already ordered tail arrivals use an append path.
Existing insertion indices, cursor selection and bulk marks remain attached to
their records even without a row-ID callback. The return count includes incoming
records hidden by filtering or search.

`retain_rows(predicate)` removes records that fail the predicate. It evaluates
every resident row once in insertion order, including hidden records. Surviving
selection and bulk marks are remapped to their new insertion indices; removed
selection clears permanently. It retains existing visible ordering without
rerunning search formatting or sort comparators. Temporary index storage is
O(resident rows). If the predicate panics, no rows have been removed yet.

```rust
use ratagrid::{Column, Grid};
let mut grid = Grid::new(vec![Column::new("ID", 8, |n: &u64| n.to_string())], vec![1, 2]);
grid.select_row(1);
assert_eq!(grid.append_rows([3, 4]).unwrap(), 2);
assert_eq!(grid.retain_rows(|id| *id >= 2).unwrap(), 1);
assert_eq!(grid.model().selected(), Some(&2));
assert_eq!(grid.model().selected_index(), Some(0));
```

After removal, discard cached insertion indices in application code. Client page
counts and page clamping update immediately, before rendering. Append preserves
cell flashes; removal clears them to avoid highlighting the wrong record.
Empty appends and retention without removals preserve interaction state. Both
operations close stale cell details when the dataset changes; render again before
handling mouse input. These APIs are also available directly on `GridModel`.

External grids reject both operations with `ExternalRowsError` before consuming
an iterator or invoking a predicate. Change the source and reload its page instead;
locally adding or deleting records could invalidate its totals and cursor tokens.

## Drive animation from your event loop

Ratagrid owns no timer or event loop. After marking a cell with `flash_cell(row_index, column_index, duration)`, call `advance_animations(elapsed_frame_time)` before drawing. Use `is_animating()` to choose a shorter event polling interval, and redraw after the final advance even when it returns false, so the base style is restored. The playground polls at 30 ms while animating and 100 ms otherwise.

Cell flashes follow insertion indices across owned sorting and client paging. Repeated flashes restart that cell's fade. Replacing data or requesting an external page clears them. Invalid cells and zero durations are rejected. `clear_animations()` removes all flashes immediately.

Customize `grid.style_mut().flash` to match your theme. RGB foreground/background colors interpolate toward each cell's current normal, selected or hover style. With non-RGB colors, the highlight switches back halfway through. Neighboring cells keep their own styles. The library animates styling; numeric interpolation is application logic in the playground's `RowTween` and `tick_at` methods.

For reduced motion, apply data edits immediately and skip `flash_cell`, or clear active highlights. The playground's A control does both.

## Export a reproducible frame

```sh
cargo run --release --locked --example playground -- \
  --snapshot boost.svg --animation-frame 250
```

`--animation-frame` requires `--snapshot` and renders the initial Boost interaction at the given elapsed milliseconds. This uses the same drawing and interpolation code as the interactive example, without sleeping or opening a terminal. The animated preview above was assembled from these rendered frames.

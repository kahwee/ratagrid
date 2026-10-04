# Updates and animation

A click anywhere in your application can change a grid record. Ratagrid emits input actions; your event loop decides what they change. The playground's **Boost** button sits below the table and updates three cells in the selected row, or the first visible row if no record on this page is selected.

```sh
cargo run --release --example playground
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

Owned grids preserve selection, maintain the active global sort and retain stable ties in both directions. They reposition only the edited row, using logarithmic comparator calls and up to linear index lookup/movement. Updating an unsorted row requires no ordering scan. A changed sort key can move a row out of the current viewport or client page; navigation still follows the selected record.

External grids update only the current resident page, preserving the source's order and skipping local comparators. The edit is local: persist it through your API/database, then reload the page if server ordering or totals have changed. External navigation clears animations. With a row ID callback, selection is restored if the next accepted page contains the selected ID exactly once; otherwise selection clears. Application value tweens must also be cancelled when a page is replaced; the playground demonstrates this.

## Drive animation from your event loop

Ratagrid owns no timer or event loop. After marking a cell with `flash_cell(row_index, column_index, duration)`, call `advance_animations(elapsed_frame_time)` before drawing. Use `is_animating()` to choose a shorter event polling interval, and redraw after the final advance even when it returns false, so the base style is restored. The playground polls at 30 ms while animating and 100 ms otherwise.

Cell flashes follow insertion indices across owned sorting and client paging. Repeated flashes restart that cell's fade. Replacing data or requesting an external page clears them. Invalid cells and zero durations are rejected. `clear_animations()` removes all flashes immediately.

Customize `grid.style_mut().flash` to match your theme. RGB foreground/background colors interpolate toward each cell's current normal, selected or hover style. With non-RGB colors, the highlight switches back halfway through. Neighboring cells keep their own styles. The library animates styling; numeric interpolation is application logic in the playground's `RowTween` and `tick_at` methods.

For reduced motion, apply data edits immediately and skip `flash_cell`, or clear active highlights. The playground's A control does both.

## Export a reproducible frame

```sh
cargo run --release --example playground -- \
  --snapshot boost.svg --animation-frame 250
```

`--animation-frame` requires `--snapshot` and renders the initial Boost interaction at the given elapsed milliseconds. This uses the same drawing and interpolation code as the interactive example, without sleeping or opening a terminal. The animated preview above was assembled from these rendered frames.

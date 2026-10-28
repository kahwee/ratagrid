//! Reproducible, in-memory comparisons for streaming applications.
//! Run: cargo run --release --locked --example live_updates
use ratagrid::{Column, GridModel};
use std::{
    hint::black_box,
    time::{Duration, Instant},
};

fn model(n: usize) -> GridModel<i64> {
    GridModel::new(
        vec![
            Column::new("Value", 16, |n: &i64| format!("東京 value {n}")).sortable(|a, b| a.cmp(b)),
        ],
        (0..n as i64).collect(),
    )
}
fn report(name: &str, rows: usize, mut sample: impl FnMut() -> Duration) {
    let mut times: Vec<_> = (0..5).map(|_| sample()).collect();
    times.sort();
    println!("{name},{rows},{:.6}", times[2].as_secs_f64() * 1000.0);
}
fn main() {
    println!("scenario,rows,median_ms");
    for n in [10_000, 100_000] {
        for batch in [false, true] {
            report(
                if batch {
                    "batch_1000"
                } else {
                    "sequential_1000"
                },
                n,
                || {
                    let mut grid = model(n);
                    grid.toggle_sort(0);
                    let start = Instant::now();
                    if batch {
                        grid.update_rows(0..1000, |_, row| *row += n as i64);
                    } else {
                        for i in 0..1000 {
                            grid.update_row(i, |row| *row += n as i64);
                        }
                    }
                    let elapsed = start.elapsed();
                    let _ = black_box(grid);
                    elapsed
                },
            );
        }
        for hashed in [false, true] {
            report(
                if hashed {
                    "hashed_restore_500"
                } else {
                    "equality_restore_500"
                },
                n,
                || {
                    let mut grid = if hashed {
                        model(n).with_hashed_row_id(|n| *n)
                    } else {
                        model(n).with_row_id(|n| *n)
                    };
                    grid.set_selected_indices(0..500);
                    let replacement = (0..n as i64).rev().collect();
                    let start = Instant::now();
                    grid.replace_rows(replacement);
                    let elapsed = start.elapsed();
                    let _ = black_box(grid);
                    elapsed
                },
            );
        }
        for rebuild in [false, true] {
            report(
                if rebuild {
                    "filtered_edit_full_refresh"
                } else {
                    "filtered_edit_incremental"
                },
                n,
                || {
                    let mut grid = model(n);
                    grid.set_search("東京");
                    grid.toggle_sort(0);
                    let start = Instant::now();
                    grid.update_row(0, |row| *row += n as i64);
                    if rebuild {
                        grid.set_search("東京");
                    }
                    let elapsed = start.elapsed();
                    let _ = black_box(grid);
                    elapsed
                },
            );
        }
        report("unicode_search", n, || {
            let mut grid = model(n);
            let start = Instant::now();
            grid.set_search("東京 VALUE 99");
            let elapsed = start.elapsed();
            let _ = black_box(grid);
            elapsed
        });
    }
}

//! Reproducible release-mode stress measurements. No terminal I/O or external database.
use super::data::{Record, Scenario, columns, fulfill_page, records};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratagrid::Grid;
use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};
use std::{hint::black_box, mem::size_of};
use std::{
    num::NonZeroUsize,
    time::{Duration, Instant},
};

struct Stats {
    samples: usize,
    median: f64,
    p95: f64,
    max: f64,
}
impl Stats {
    fn from(mut samples: Vec<f64>) -> Self {
        samples.sort_by(f64::total_cmp);
        let count = samples.len();
        Self {
            samples: count,
            median: samples[count / 2],
            p95: samples[(count * 95).div_ceil(100) - 1],
            max: samples[count - 1],
        }
    }
}
fn measure(count: usize, mut operation: impl FnMut()) -> Stats {
    for _ in 0..3 {
        operation();
    }
    Stats::from(
        (0..count)
            .map(|_| {
                let start = Instant::now();
                operation();
                start.elapsed().as_secs_f64() * 1000.0
            })
            .collect(),
    )
}
struct Case {
    name: &'static str,
    total: usize,
    columns: usize,
    area: Rect,
    external_size: Option<usize>,
    unicode: bool,
}
impl Case {
    fn emit(&self, resident: usize, operation: &str, stats: Stats) {
        println!(
            "{},{},{},{},{},{},{},{},{:.6},{:.6},{:.6},{}",
            self.name,
            self.total,
            resident,
            self.columns,
            self.area.width,
            self.area.height,
            operation,
            stats.samples,
            stats.median,
            stats.p95,
            stats.max,
            resident * (size_of::<Record>() + size_of::<usize>())
        );
    }
}
fn key(grid: &mut Grid<Record>, code: KeyCode) {
    black_box(grid.handle_event(&Event::Key(KeyEvent::new(code, KeyModifiers::NONE))));
}
fn focus(grid: &mut Grid<Record>, column: usize) {
    key(grid, KeyCode::Esc);
    for _ in 0..=column {
        key(grid, KeyCode::Tab);
    }
}
fn full_sort(grid: &mut Grid<Record>, column: usize) -> Stats {
    focus(grid, column);
    let samples = (0..3)
        .map(|_| {
            let start = Instant::now();
            key(grid, KeyCode::Enter);
            let elapsed = start.elapsed().as_secs_f64() * 1000.0;
            // Reset outside the sample: descending, then insertion order.
            key(grid, KeyCode::Enter);
            key(grid, KeyCode::Enter);
            elapsed
        })
        .collect();
    key(grid, KeyCode::Esc);
    Stats::from(samples)
}
fn run_case(case: Case) {
    eprintln!(
        "Stress: {} ({} total, {}x{}, {} columns)",
        case.name, case.total, case.area.width, case.area.height, case.columns
    );
    let mut grid = if let Some(size) = case.external_size {
        let mut grid = Grid::new_paged(
            columns(case.columns),
            case.total,
            NonZeroUsize::new(size).unwrap(),
        );
        fulfill_page(&mut grid, 0, case.total);
        grid
    } else {
        Grid::new(
            columns(case.columns),
            records(
                if case.unicode {
                    Scenario::Unicode
                } else {
                    Scenario::Jobs
                },
                case.total,
                0,
            ),
        )
    };
    let resident = grid.model().rows().len();
    let mut buffer = Buffer::empty(case.area);
    grid.widget().render(case.area, &mut buffer);
    key(&mut grid, KeyCode::End);
    case.emit(
        resident,
        "render_selected",
        measure(100, || {
            grid.widget().render(case.area, &mut buffer);
            black_box(&buffer);
        }),
    );
    let selected = grid.model().selected_index().expect("nonempty case");
    case.emit(
        resident,
        "update_unsorted_selected",
        measure(100, || {
            black_box(grid.update_row(selected, |row| row.memory += 1));
        }),
    );
    for position in resident.saturating_sub(100)..resident {
        let index = grid.model().index_at(position).unwrap();
        for column in [3, 4, 5] {
            grid.flash_cell(index, column, Duration::from_secs(3600));
        }
    }
    case.emit(
        resident,
        "frame_100_flashing_rows",
        measure(100, || {
            grid.advance_animations(Duration::from_millis(16));
            grid.widget().render(case.area, &mut buffer);
            black_box(&buffer);
        }),
    );
    grid.clear_animations();
    if case.external_size.is_some() {
        case.emit(
            resident,
            "fetch_virtual_page_and_render",
            measure(100, || {
                let next = (grid.page_state().unwrap().page + 1)
                    % grid.page_state().unwrap().page_count.unwrap();
                grid.set_page(next);
                fulfill_page(&mut grid, 0, case.total);
                grid.widget().render(case.area, &mut buffer);
                black_box(&buffer);
            }),
        );
        let original_size = resident;
        assert_eq!(grid.model().rows().len(), original_size);
        case.emit(
            resident,
            "external_50_updates_and_render",
            measure(60, || {
                for index in 0..50.min(resident) {
                    grid.update_row(index, |row| {
                        row.latency = Some(row.latency.unwrap_or(0) + 1)
                    });
                }
                grid.widget().render(case.area, &mut buffer);
                black_box(&buffer);
            }),
        );
        return;
    }
    case.emit(resident, "sort_latency_ascending", full_sort(&mut grid, 3));
    case.emit(resident, "sort_workload_ascending", full_sort(&mut grid, 1));
    focus(&mut grid, 3);
    key(&mut grid, KeyCode::Enter);
    key(&mut grid, KeyCode::Esc);
    key(&mut grid, KeyCode::End);
    let selected = grid.model().selected_index().unwrap();
    case.emit(
        resident,
        "sorted_selected_unchanged_key",
        measure(100, || {
            grid.update_row(selected, |row| row.memory += 1);
        }),
    );
    let mut high = false;
    case.emit(
        resident,
        "sorted_selected_cross_dataset",
        measure(30, || {
            high = !high;
            grid.update_row(selected, |row| {
                row.latency = Some(if high { u64::MAX } else { 0 })
            });
        }),
    );
    // Time a different unselected row near the end, where index lookup scans most of the order.
    let other = grid.model().index_at(resident - 2).unwrap();
    case.emit(
        resident,
        "sorted_unselected_unchanged_key",
        measure(60, || {
            grid.update_row(other, |row| row.memory += 1);
        }),
    );
    let indices: Vec<usize> = (resident.saturating_sub(50)..resident)
        .filter(|&i| i != selected)
        .collect();
    for &index in &indices {
        for column in [3, 4, 5] {
            grid.flash_cell(index, column, Duration::from_secs(3600));
        }
    }
    case.emit(
        resident,
        "sorted_50_small_updates_and_render",
        measure(30, || {
            grid.advance_animations(Duration::from_millis(16));
            for &index in &indices {
                grid.update_row(index, |row| {
                    row.latency = Some(row.latency.unwrap_or(0).saturating_add(1))
                });
            }
            grid.widget().render(case.area, &mut buffer);
            black_box(&buffer);
        }),
    );
    case.emit(
        resident,
        "sorted_50_cross_dataset_and_render",
        measure(30, || {
            high = !high;
            grid.advance_animations(Duration::from_millis(16));
            for &index in &indices {
                grid.update_row(index, |row| {
                    row.latency = Some(if high { u64::MAX } else { 0 })
                });
            }
            grid.widget().render(case.area, &mut buffer);
            black_box(&buffer);
        }),
    );
    assert_eq!(grid.model().selected_index(), Some(selected));
}

pub(super) fn run(large: bool) {
    println!(
        "case,total_rows,resident_rows,columns,width,height,operation,samples,p50_ms,p95_ms,max_ms,record_and_order_bytes"
    );
    for total in [250, 100_000, 1_000_000, 2_000_000]
        .into_iter()
        .chain(large.then_some(10_000_000))
    {
        run_case(Case {
            name: "owned",
            total,
            columns: 9,
            area: Rect::new(0, 0, 132, 34),
            external_size: None,
            unicode: false,
        });
    }
    run_case(Case {
        name: "unicode",
        total: 128,
        columns: 9,
        area: Rect::new(0, 0, 132, 34),
        external_size: None,
        unicode: true,
    });
    run_case(Case {
        name: "wide",
        total: 2_000_000,
        columns: 128,
        area: Rect::new(0, 0, 132, 34),
        external_size: None,
        unicode: false,
    });
    run_case(Case {
        name: "max_viewport",
        total: 2_000_000,
        columns: 128,
        area: Rect::new(0, 0, 500, 200),
        external_size: None,
        unicode: false,
    });
    for (size, area) in [
        (50, Rect::new(0, 0, 132, 34)),
        (500, Rect::new(0, 0, 500, 200)),
    ] {
        run_case(Case {
            name: "external_virtual",
            total: 100_000_000,
            columns: 9,
            area,
            external_size: Some(size),
            unicode: false,
        });
    }
}

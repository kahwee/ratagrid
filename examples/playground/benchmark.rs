//! Owned-data rendering and sorting benchmark.
use super::{
    data::{Record, SCENARIOS, Scenario, columns, records},
    options::Options,
};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratagrid::Grid;
use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};
use std::time::Instant;

pub(super) fn benchmark(options: &Options) {
    println!(
        "scenario,rows,columns,render_initial_ms,render_selected_ms,sort_numeric_ms,sort_name_ms,visible_cell_upper_bound"
    );
    for scenario in SCENARIOS.into_iter().filter(|s| *s != Scenario::Paged) {
        let count = options.rows.unwrap_or(scenario.rows());
        let column_count = options.columns.unwrap_or(scenario.columns());
        let mut grid = Grid::new(columns(column_count), records(scenario, count, 0));
        let area = Rect::new(0, 0, options.width, options.height);
        let mut buffer = Buffer::empty(area);
        grid.widget().render(area, &mut buffer);
        let measure = |grid: &mut Grid<Record>, buffer: &mut Buffer| {
            let start = Instant::now();
            for _ in 0..40 {
                grid.widget().render(area, buffer);
            }
            start.elapsed().as_secs_f64() * 1000.0 / 40.0
        };
        let initial = measure(&mut grid, &mut buffer);
        grid.handle_event(&Event::Key(KeyEvent::new(KeyCode::End, KeyModifiers::NONE)));
        let selected = measure(&mut grid, &mut buffer);
        for _ in 0..4.min(column_count) {
            grid.handle_event(&Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)));
        }
        let start = Instant::now();
        grid.handle_event(&Event::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::NONE,
        )));
        let numeric = start.elapsed().as_secs_f64() * 1000.0;
        grid.handle_event(&Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)));
        for _ in 0..2.min(column_count) {
            grid.handle_event(&Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)));
        }
        let start = Instant::now();
        grid.handle_event(&Event::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::NONE,
        )));
        let name = start.elapsed().as_secs_f64() * 1000.0;
        println!(
            "{},{count},{column_count},{initial:.4},{selected:.4},{numeric:.3},{name:.3},{}",
            scenario.slug(),
            count.min(usize::from(options.height.saturating_sub(1))) * column_count
        );
    }
}

//! Repeatable end-to-end long-cell rendering benchmark on stable Rust.
//! cargo run --release --locked --example long_cells
use ratagrid::{Column, Grid};
use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};
use std::{hint::black_box, time::Instant};

fn main() {
    println!("case,bytes,column_cells,iterations,ns_per_render");
    for (name, text) in [
        ("ascii_short", "a".repeat(40)),
        ("ascii_4k", "a".repeat(4096)),
        ("ascii_1m", "a".repeat(1_048_576)),
        ("unicode_1m", "東京 e\u{301} 👩🏽‍💻 ".repeat(40_000)),
        ("bidi_1m", "abc\u{202e}e\u{301}\u{2069} ".repeat(65_536)),
        (
            "controls_1m",
            format!("{}tail", "\u{202e}\x1b\n".repeat(200_000)),
        ),
    ] {
        let bytes = text.len();
        let mut grid = Grid::new(
            vec![Column::new("Value", 21, |s: &String| s.clone())],
            vec![text],
        );
        let area = Rect::new(0, 0, 21, 3);
        let mut buffer = Buffer::empty(area);
        for _ in 0..10 {
            grid.widget().render(area, &mut buffer);
        }
        let iterations = if bytes > 100_000 { 100 } else { 2_000 };
        let start = Instant::now();
        for _ in 0..iterations {
            grid.widget().render(area, &mut buffer);
            black_box(&buffer);
        }
        println!(
            "{name},{bytes},20,{iterations},{:.1}",
            start.elapsed().as_nanos() as f64 / iterations as f64
        );
    }
}

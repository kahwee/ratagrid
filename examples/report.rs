//! Export filtered resident data without a terminal or filesystem dependency.
use ratagrid::{Column, CsvOptions, Grid};
use std::io::{self, Write};

fn main() -> io::Result<()> {
    let mut grid = Grid::new(
        vec![
            Column::new("Project", 12, |row: &(&str, u32)| row.0.into()),
            Column::new("Open issues", 12, |row: &(&str, u32)| row.1.to_string()),
        ],
        vec![("Terminal, tools", 12), ("東京", 4), ("Archived", 0)],
    );
    grid.set_filter(|row| row.1 > 0).expect("owned grid");
    grid.autosize_columns(100, 40);
    let mut output = io::BufWriter::new(io::stdout().lock());
    grid.write_csv(&mut output, CsvOptions::default())?;
    output.flush()
}

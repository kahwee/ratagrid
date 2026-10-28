//! Streaming exports of the application's resident data.
use super::*;
use std::io::{self, Write};

/// Which resident records to export, in filtered/sorted display order.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CsvScope {
    /// All filtered resident rows, across client-side pages.
    #[default]
    Filtered,
    /// Only the current page.
    CurrentPage,
    /// Explicit bulk marks that pass the filter; no active-row fallback.
    Selected,
}

/// CSV uses visible columns in display order and full display-formatted values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CsvOptions {
    pub scope: CsvScope,
    pub include_headers: bool,
}

impl Default for CsvOptions {
    fn default() -> Self {
        Self {
            scope: CsvScope::Filtered,
            include_headers: true,
        }
    }
}

impl<T> Grid<T> {
    /// Stream CSV, returning the number of data records written (not headers).
    ///
    /// Fields are quoted, embedded quotes doubled, and records end with CRLF.
    /// Unicode, tabs and embedded newlines survive; other terminal/bidi controls
    /// are removed. Search aliases do not replace display values.
    ///
    /// External grids export only their resident page, never fetch more data,
    /// and reject loading (`WouldBlock`) or failed (`Other`) states before writing.
    /// No visible columns produces no output. Empty data can still emit headers.
    /// Writer failures may leave partial output; applications own buffering and
    /// atomic file replacement. Use a `BufWriter` when writing files.
    pub fn write_csv<W: Write>(&self, writer: &mut W, options: CsvOptions) -> io::Result<usize> {
        match self.load_state() {
            LoadState::Loading => {
                return Err(io::Error::new(io::ErrorKind::WouldBlock, "grid is loading"));
            }
            LoadState::Error(message) => return Err(io::Error::other(message.to_owned())),
            _ => {}
        }
        let columns = self.visible_columns();
        if columns.is_empty() {
            return Ok(0);
        }
        if options.include_headers {
            for (position, &column) in columns.iter().enumerate() {
                if position != 0 {
                    writer.write_all(b",")?;
                }
                write_field(writer, &self.model.columns[column].title)?;
            }
            writer.write_all(b"\r\n")?;
        }
        let range = if options.scope == CsvScope::CurrentPage {
            self.row_range()
        } else {
            0..self.model.visible_len()
        };
        let mut count = 0;
        for &row in &self.model.order[range] {
            if options.scope == CsvScope::Selected && !self.model.is_row_selected(row) {
                continue;
            }
            for (position, &column) in columns.iter().enumerate() {
                if position != 0 {
                    writer.write_all(b",")?;
                }
                write_field(
                    writer,
                    &(self.model.columns[column].format)(&self.model.rows()[row]),
                )?;
            }
            writer.write_all(b"\r\n")?;
            count += 1;
        }
        Ok(count)
    }
}

fn write_field<W: Write>(writer: &mut W, value: &str) -> io::Result<()> {
    writer.write_all(b"\"")?;
    let mut start = 0;
    for (index, ch) in value.char_indices() {
        if ch == '"' || (text::is_unsafe(ch) && !matches!(ch, '\r' | '\n' | '\t')) {
            writer.write_all(&value.as_bytes()[start..index])?;
            if ch == '"' {
                writer.write_all(b"\"\"")?;
            }
            start = index + ch.len_utf8();
        }
    }
    writer.write_all(&value.as_bytes()[start..])?;
    writer.write_all(b"\"")
}

//! Headless SVG export of a rendered terminal buffer.
use ratatui::{
    buffer::{Buffer, Cell},
    style::{Color, Modifier},
};
use std::{fmt::Write as _, fs, io, path::PathBuf};
use unicode_width::UnicodeWidthStr;

fn color(color: Color, fallback: &str) -> String {
    match color {
        Color::Rgb(r, g, b) => format!("#{r:02x}{g:02x}{b:02x}"),
        Color::Black => "#000000".into(),
        Color::White => "#ffffff".into(),
        Color::Cyan => "#69e7c1".into(),
        Color::DarkGray => "#26384a".into(),
        _ => fallback.into(),
    }
}
// Resolve terminal reverse-video before grouping SVG colour runs.
fn cell_colors(cell: &Cell) -> (Color, Color) {
    if cell.modifier.contains(Modifier::REVERSED) {
        (cell.bg, cell.fg)
    } else {
        (cell.fg, cell.bg)
    }
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
pub(super) fn snapshot(buffer: &Buffer, path: &PathBuf) -> io::Result<()> {
    let cell_width = 10;
    let cell_height = 20;
    let padding = 24;
    let width = u32::from(buffer.area.width) * cell_width + padding * 2;
    let height = u32::from(buffer.area.height) * cell_height + padding * 2;
    let background = color(
        buffer.content.first().map_or(Color::Reset, |cell| cell.bg),
        "#0c121c",
    );
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\"><rect width=\"100%\" height=\"100%\" fill=\"{background}\"/><g font-family=\"DejaVu Sans Mono, monospace\" font-size=\"16\">"
    );
    // Emit runs so snapshots stay small enough to review and commit.
    for y in 0..buffer.area.height {
        let mut x = 0;
        while x < buffer.area.width {
            let start = x;
            let bg = cell_colors(&buffer[(x, y)]).1;
            while x < buffer.area.width && cell_colors(&buffer[(x, y)]).1 == bg {
                x += 1;
            }
            if color(bg, "#0c121c") == background {
                continue;
            }
            let px = u32::from(start) * cell_width + padding;
            let py = u32::from(y) * cell_height + padding;
            let run_width = u32::from(x - start) * cell_width;
            let _ = write!(
                svg,
                "<rect x=\"{px}\" y=\"{py}\" width=\"{run_width}\" height=\"{cell_height}\" fill=\"{}\"/>",
                color(bg, "#0c121c")
            );
        }
    }
    for y in 0..buffer.area.height {
        let mut x = 0;
        while x < buffer.area.width {
            let start = x;
            let fg = cell_colors(&buffer[(x, y)]).0;
            let bold = buffer[(x, y)].modifier.contains(Modifier::BOLD);
            let mut text = String::new();
            while x < buffer.area.width
                && cell_colors(&buffer[(x, y)]).0 == fg
                && buffer[(x, y)].modifier.contains(Modifier::BOLD) == bold
            {
                let symbol = buffer[(x, y)].symbol();
                text.push_str(symbol);
                x = x.saturating_add(UnicodeWidthStr::width(symbol).max(1) as u16);
            }
            if text.trim().is_empty() {
                continue;
            }
            let px = u32::from(start) * cell_width + padding;
            let py = u32::from(y) * cell_height + padding + 16;
            let run_width = u32::from(x - start) * cell_width;
            let _ = write!(
                svg,
                "<text xml:space=\"preserve\" x=\"{px}\" y=\"{py}\" fill=\"{}\" textLength=\"{run_width}\" lengthAdjust=\"spacingAndGlyphs\"{}>{}</text>",
                color(fg, "#dde8f3"),
                if bold { " font-weight=\"bold\"" } else { "" },
                escape(&text)
            );
        }
    }
    svg.push_str("</g></svg>\n");
    fs::write(path, svg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{layout::Rect, style::Style};

    #[test]
    fn svg_preserves_light_canvas_and_reversed_cell_colors() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 2, 1));
        buffer[(0, 0)].set_symbol("A").set_style(
            Style::default()
                .fg(Color::Rgb(24, 38, 52))
                .bg(Color::Rgb(246, 248, 252)),
        );
        buffer[(1, 0)].set_symbol("B").set_style(
            Style::default()
                .fg(Color::Rgb(105, 231, 193))
                .bg(Color::Rgb(12, 18, 28))
                .add_modifier(Modifier::REVERSED | Modifier::BOLD),
        );
        let path = std::env::temp_dir().join(format!("ratagrid-export-{}.svg", std::process::id()));
        snapshot(&buffer, &path).unwrap();
        let svg = fs::read_to_string(&path).unwrap();
        fs::remove_file(path).unwrap();
        assert!(svg.contains("width=\"100%\" height=\"100%\" fill=\"#f6f8fc\""));
        assert!(svg.contains("x=\"34\" y=\"24\" width=\"10\" height=\"20\" fill=\"#69e7c1\""));
        assert!(svg.contains("fill=\"#0c121c\" textLength=\"10\" lengthAdjust=\"spacingAndGlyphs\" font-weight=\"bold\">B</text>"));
    }
}

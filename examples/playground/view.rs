//! Dashboard layout and rendering.
use super::{
    app::{ACCENT, App, BACKGROUND, Control, INK, MUTED, TWEEN_TIME, grouped},
    data::{SCENARIOS, Scenario},
};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
};
use std::time::Instant;
use unicode_width::UnicodeWidthStr;

impl App {
    pub(super) fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();
        self.buttons.clear();
        let (bg, ink, muted, accent) = if self.light {
            (
                Color::Rgb(246, 248, 252),
                Color::Rgb(24, 38, 52),
                Color::Rgb(82, 106, 125),
                Color::Rgb(0, 110, 85),
            )
        } else {
            (BACKGROUND, INK, MUTED, ACCENT)
        };
        frame.render_widget(Block::new().style(Style::default().bg(bg).fg(ink)), area);
        let compact = area.height < 24;
        let sections = Layout::vertical([
            Constraint::Length(if compact { 2 } else { 3 }),
            Constraint::Length(1),
            Constraint::Length(if compact { 0 } else { 1 }),
            Constraint::Length(if compact { 1 } else { 3 }),
            Constraint::Min(2),
            Constraint::Length(if compact { 2 } else { 3 }),
            Constraint::Length(2),
        ])
        .split(area);
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(vec![
                    Span::styled(
                        " RATAGRID ",
                        Style::default()
                            .fg(bg)
                            .bg(accent)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        "  /  PLAYGROUND",
                        Style::default().fg(ink).add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(Span::styled(
                    format!(" {}", self.scenario.description()),
                    Style::default().fg(muted),
                )),
            ]),
            sections[0],
        );
        let mut x = sections[1].x;
        for (index, scenario) in SCENARIOS.iter().enumerate() {
            let label = format!(
                " {} {} ",
                index + 1,
                if area.width < 90 {
                    scenario.compact_label()
                } else {
                    scenario.label()
                }
            );
            let width = UnicodeWidthStr::width(label.as_str()) as u16;
            if x >= sections[1].right() {
                break;
            }
            let button = Rect::new(
                x,
                sections[1].y,
                width.min(sections[1].right() - x),
                sections[1].height,
            );
            let style = if *scenario == self.scenario {
                Style::default()
                    .fg(bg)
                    .bg(accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(muted)
            };
            frame.render_widget(Paragraph::new(label).style(style), button);
            self.buttons.push((button, Control::Scenario(*scenario)));
            x += width + 1;
        }
        if compact {
            frame.render_widget(
                Paragraph::new(format!(
                    " {} rows · {:.3} ms draw · {:.1} ms sort",
                    grouped(self.count),
                    self.draw_ms,
                    self.sort_ms
                ))
                .style(Style::default().fg(accent)),
                sections[3],
            );
        } else {
            let metrics = Layout::horizontal([
                Constraint::Percentage(34),
                Constraint::Percentage(33),
                Constraint::Percentage(33),
            ])
            .split(sections[3]);
            for (rect, title, value) in [
                (
                    metrics[0],
                    " ROWS ",
                    if self.scenario == Scenario::Paged {
                        format!(
                            " {} total / {} loaded",
                            grouped(self.count),
                            self.grid.model().rows().len()
                        )
                    } else {
                        format!(" {}  /  {} cols", grouped(self.count), self.column_count)
                    },
                ),
                (
                    metrics[1],
                    " GRID DRAW ",
                    format!(" {:.3} ms", self.draw_ms),
                ),
                (
                    metrics[2],
                    if self.scenario == Scenario::Paged {
                        " PAGE LOAD "
                    } else {
                        " LAST SORT "
                    },
                    format!(" {:.2} ms", self.sort_ms),
                ),
            ] {
                frame.render_widget(
                    Paragraph::new(value)
                        .block(
                            Block::bordered()
                                .title(title)
                                .border_style(Style::default().fg(muted)),
                        )
                        .style(Style::default().fg(accent)),
                    rect,
                );
            }
        }
        let block = Block::bordered()
            .title(if self.tweens.is_empty() {
                format!(" {} ", self.scenario.label())
            } else {
                format!(
                    " {} · updating {} row(s) ",
                    self.scenario.label(),
                    self.tweens.len()
                )
            })
            .border_style(Style::default().fg(muted));
        let inner = block.inner(sections[4]);
        frame.render_widget(block, sections[4]);
        let start = Instant::now();
        frame.render_widget(self.grid.widget(), inner);
        self.draw_ms = start.elapsed().as_secs_f64() * 1000.0;
        if self.count == 0 && inner.height > 2 {
            frame.render_widget(
                Paragraph::new("No records yet. Click Live below, or press L.")
                    .style(Style::default().fg(muted)),
                Rect::new(inner.x + 1, inner.y + 2, inner.width.saturating_sub(2), 1),
            );
        }
        let selected = self
            .grid
            .model()
            .selected()
            .map_or("none".into(), |r| format!("#{:06} {}", r.id + 1, r.owner()));
        let progress = self.tweens.last().map_or(String::new(), |tween| {
            let fraction = (self
                .last_tick
                .saturating_duration_since(tween.started)
                .as_secs_f64()
                / TWEEN_TIME.as_secs_f64())
            .min(1.0);
            let filled = (fraction * 10.0).floor() as usize;
            format!(
                " · [{}{}] {:>3}%",
                "━".repeat(filled),
                "─".repeat(10 - filled),
                (fraction * 100.0) as usize
            )
        });
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(Span::styled(
                    format!(" {}", self.status),
                    Style::default().fg(ink),
                )),
                Line::from(Span::styled(
                    format!(
                        " {}×{} viewport · row {} · horizontal {} cells · selected {selected}{progress}",
                        area.width,
                        area.height,
                        self.grid.row_offset() + 1,
                        self.grid.column_offset()
                    ),
                    Style::default().fg(muted),
                )),
            ]),
            sections[5],
        );
        let mut x = sections[6].x;
        for (label, control) in [
            (" [B] Boost ", Control::Boost),
            (
                if self.motion {
                    " [A] Motion ON "
                } else {
                    " [A] Motion OFF "
                },
                Control::Motion,
            ),
            (
                if self.live {
                    " [L] Live ON "
                } else {
                    " [L] Live OFF "
                },
                Control::Live,
            ),
            (
                if self.scenario == Scenario::Paged {
                    " [P] Page size "
                } else {
                    " [P] Pages "
                },
                Control::Pagination,
            ),
            (" [T] Theme ", Control::Theme),
            (" [R] Reset ", Control::Reset),
            (" [Q] Quit ", Control::Quit),
        ] {
            if x >= sections[6].right() {
                break;
            }
            let width = (label.len() as u16).min(sections[6].right() - x);
            let button = Rect::new(x, sections[6].y, width, sections[6].height.min(1));
            frame.render_widget(
                Paragraph::new(label).style(if matches!(control, Control::Boost) {
                    Style::default()
                        .fg(bg)
                        .bg(accent)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(accent)
                }),
                button,
            );
            self.buttons.push((button, control));
            x += width;
        }
        if sections[6].height > 1 {
            frame.render_widget(Paragraph::new(" Click Boost: animate other cells · Click headers: sort · Drag header │ / ↔: resize · Wheel: scroll").style(Style::default().fg(muted)), Rect::new(sections[6].x, sections[6].y + 1, sections[6].width, 1));
        }
    }
}

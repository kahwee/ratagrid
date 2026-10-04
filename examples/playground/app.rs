//! Application state, controls, live updates and value animation.
use super::data::{Record, SCENARIOS, Scenario, fulfill_page, make_grid, records};
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind};
use ratagrid::{Action, Grid, SortDirection};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
};
use std::{
    cell::Cell,
    num::NonZeroUsize,
    rc::Rc,
    time::{Duration, Instant},
};

pub(super) const BACKGROUND: Color = Color::Rgb(12, 18, 28);
pub(super) const PANEL: Color = Color::Rgb(18, 28, 41);
pub(super) const INK: Color = Color::Rgb(221, 232, 243);
pub(super) const MUTED: Color = Color::Rgb(128, 149, 170);
pub(super) const ACCENT: Color = Color::Rgb(105, 231, 193);
pub(super) struct App {
    pub(super) grid: Grid<Record>,
    pub(super) scenario: Scenario,
    pub(super) count: usize,
    pub(super) column_count: usize,
    pub(super) buttons: Vec<(Rect, Control)>,
    pub(super) generation: u64,
    pub(super) live: bool,
    pub(super) light: bool,
    palette: Rc<Cell<bool>>,
    pub(super) draw_ms: f64,
    pub(super) sort_ms: f64,
    pub(super) status: String,
    pub(super) last_feed: Instant,
    pub(super) last_tick: Instant,
    pub(super) motion: bool,
    pub(super) tweens: Vec<RowTween>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Values {
    pub(super) latency: u64,
    pub(super) delta: i64,
    pub(super) memory: u64,
}
impl Values {
    pub(super) fn from_record(row: &Record) -> Self {
        Self {
            latency: row.latency.unwrap_or(0),
            delta: row.delta,
            memory: row.memory,
        }
    }
    pub(super) fn write(self, row: &mut Record) {
        row.latency = Some(self.latency);
        row.delta = self.delta;
        row.memory = self.memory;
    }
    pub(super) fn interpolate(self, end: Self, progress: f64) -> Self {
        Self {
            latency: self.latency + ((end.latency - self.latency) as f64 * progress).round() as u64,
            delta: self.delta + ((end.delta - self.delta) as f64 * progress).round() as i64,
            memory: self.memory + ((end.memory - self.memory) as f64 * progress).round() as u64,
        }
    }
}
pub(super) struct RowTween {
    pub(super) index: usize,
    pub(super) id: usize,
    pub(super) start: Values,
    pub(super) end: Values,
    pub(super) started: Instant,
}
pub(super) const TWEEN_TIME: Duration = Duration::from_millis(650);
pub(super) const FLASH_TIME: Duration = Duration::from_millis(1100);
#[derive(Clone, Copy)]
pub(super) enum Control {
    Scenario(Scenario),
    Live,
    Pagination,
    Theme,
    Reset,
    Quit,
    Boost,
    Motion,
}
impl App {
    pub(super) fn new(scenario: Scenario, count: usize, column_count: usize) -> Self {
        let palette = Rc::new(Cell::new(false));
        let mut app = Self {
            grid: make_grid(scenario, count, column_count, 0, palette.clone()),
            scenario,
            count,
            column_count,
            buttons: vec![],
            generation: 0,
            live: false,
            light: false,
            palette,
            draw_ms: 0.0,
            sort_ms: 0.0,
            status: "Ready. Mouse and keyboard both work.".into(),
            last_feed: Instant::now(),
            last_tick: Instant::now(),
            motion: true,
            tweens: vec![],
        };
        app.theme();
        app
    }
    pub(super) fn theme(&mut self) {
        self.palette.set(self.light);
        let style = self.grid.style_mut();
        let (bg, ink, header, hover, selected) = if self.light {
            (
                Color::Rgb(246, 248, 252),
                Color::Rgb(24, 38, 52),
                Color::Rgb(216, 230, 239),
                Color::Rgb(224, 237, 246),
                Color::Rgb(179, 225, 211),
            )
        } else {
            (
                BACKGROUND,
                INK,
                PANEL,
                Color::Rgb(25, 42, 56),
                Color::Rgb(25, 67, 65),
            )
        };
        style.cell = Style::default().fg(ink).bg(bg);
        style.header = Style::default()
            .fg(if self.light {
                Color::Rgb(0, 93, 74)
            } else {
                ACCENT
            })
            .bg(header)
            .add_modifier(Modifier::BOLD);
        style.hover = Style::default().bg(hover);
        style.selected = Style::default().bg(selected);
        style.focused_header = style.header.bg(selected);
        style.separator = Style::default().fg(if self.light {
            Color::Rgb(175, 193, 206)
        } else {
            Color::Rgb(39, 58, 76)
        });
        style.flash = if self.light {
            Style::default()
                .fg(Color::Rgb(96, 48, 0))
                .bg(Color::Rgb(255, 211, 112))
        } else {
            Style::default()
                .fg(Color::Rgb(255, 236, 177))
                .bg(Color::Rgb(131, 81, 27))
        }
        .add_modifier(Modifier::BOLD);
    }
    pub(super) fn switch(&mut self, scenario: Scenario) {
        self.scenario = scenario;
        self.count = scenario.rows();
        self.column_count = scenario.columns();
        self.generation = 0;
        self.grid = make_grid(
            scenario,
            self.count,
            self.column_count,
            0,
            self.palette.clone(),
        );
        self.tweens.clear();
        self.sort_ms = 0.0;
        self.theme();
        self.status = scenario.description().into();
    }
    pub(super) fn control(&mut self, control: Control) -> bool {
        match control {
            Control::Scenario(scenario) => self.switch(scenario),
            Control::Boost => self.boost(Instant::now()),
            Control::Motion => {
                self.motion = !self.motion;
                if !self.motion {
                    for tween in self.tweens.drain(..) {
                        self.grid
                            .update_row(tween.index, |row| tween.end.write(row));
                    }
                    self.grid.clear_animations();
                }
                self.status = if self.motion {
                    "Animations on: values count up and changed cells fade."
                } else {
                    "Animations off: updates apply immediately."
                }
                .into();
            }
            Control::Live => {
                self.live = !self.live;
                self.last_feed = Instant::now();
                self.status = if self.live {
                    "Live feed enabled: records replaced every second; selection clears."
                } else {
                    "Live feed paused."
                }
                .into();
            }
            Control::Pagination => {
                if self.scenario == Scenario::Paged {
                    self.tweens.clear();
                    let current = self
                        .grid
                        .page_state()
                        .expect("paged source")
                        .page_size
                        .get();
                    let next = match current {
                        50 => 100,
                        100 => 25,
                        _ => 50,
                    };
                    self.grid.set_page_size(NonZeroUsize::new(next).unwrap());
                    fulfill_page(&mut self.grid, self.generation, self.count);
                    self.status = format!(
                        "Page size {next}; {} records loaded.",
                        self.grid.model().rows().len()
                    );
                } else if self.grid.page_state().is_some() {
                    self.grid.disable_pagination();
                    self.status = "Pagination off. All owned records can scroll.".into();
                } else {
                    self.grid.set_page_size(NonZeroUsize::new(50).unwrap());
                    self.status =
                        "Client pagination: 50 rows per page. Full dataset remains resident."
                            .into();
                }
            }
            Control::Theme => {
                self.light = !self.light;
                self.theme();
            }
            Control::Reset => {
                self.grid = make_grid(
                    self.scenario,
                    self.count,
                    self.column_count,
                    0,
                    self.palette.clone(),
                );
                self.tweens.clear();
                self.generation = 0;
                self.sort_ms = 0.0;
                self.theme();
                self.status = "Layout, data, sort and selection reset.".into();
            }
            Control::Quit => return true,
        }
        false
    }
    pub(super) fn handle(&mut self, event: &Event) -> bool {
        if !self.grid.is_searching()
            && let Event::Key(key) = event
            && key.kind != KeyEventKind::Release
            && !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER)
        {
            let control = match key.code {
                KeyCode::Char(c @ '1'..='7') => {
                    Some(Control::Scenario(SCENARIOS[(c as u8 - b'1') as usize]))
                }
                KeyCode::Char('l' | 'L') => Some(Control::Live),
                KeyCode::Char('p' | 'P') => Some(Control::Pagination),
                KeyCode::Char('t' | 'T') => Some(Control::Theme),
                KeyCode::Char('r' | 'R') => Some(Control::Reset),
                KeyCode::Char('q' | 'Q') => Some(Control::Quit),
                KeyCode::Char('b' | 'B') => Some(Control::Boost),
                KeyCode::Char('a' | 'A') => Some(Control::Motion),
                _ => None,
            };
            if let Some(control) = control {
                return self.control(control);
            }
        }
        if !self.grid.is_searching()
            && let Event::Mouse(mouse) = event
            && mouse.kind == MouseEventKind::Down(MouseButton::Left)
            && let Some((_, control)) = self
                .buttons
                .iter()
                .find(|(area, _)| area.contains((mouse.column, mouse.row).into()))
        {
            return self.control(*control);
        }
        let start = Instant::now();
        if let Some(action) = self.grid.handle_event(event) {
            match action {
                Action::PageRequested(request) => {
                    self.tweens.clear();
                    fulfill_page(&mut self.grid, self.generation, self.count);
                    self.sort_ms = start.elapsed().as_secs_f64() * 1000.0;
                    if let ratagrid::LoadState::Error(message) = self.grid.load_state() {
                        self.status = message.into();
                        return false;
                    }
                    let sort = request.sort.map_or("insertion order".into(), |s| {
                        format!(
                            "column {} {}",
                            s.column + 1,
                            if s.direction == SortDirection::Ascending {
                                "↑"
                            } else {
                                "↓"
                            }
                        )
                    });
                    self.status = format!(
                        "Loaded page {}: {} resident records, {sort}, {:.3} ms.",
                        request.page + 1,
                        self.grid.model().rows().len(),
                        self.sort_ms
                    );
                }
                Action::PageChanged(state) => {
                    self.status = format!(
                        "Page {} / {}. {} records remain resident.",
                        state.page + 1,
                        state.page_count.unwrap(),
                        state.loaded_rows
                    )
                }
                Action::SortChanged(sort) => {
                    self.sort_ms = start.elapsed().as_secs_f64() * 1000.0;
                    self.status = sort.map_or("Insertion order restored.".into(), |s| {
                        format!(
                            "Sorted column {} {} in {:.2} ms. Selected record stays selected.",
                            s.column + 1,
                            if s.direction == SortDirection::Ascending {
                                "↑"
                            } else {
                                "↓"
                            },
                            self.sort_ms
                        )
                    });
                }
                Action::SelectionChanged(index) => {
                    self.status = format!(
                        "Selected record #{:06}. Try sorting to see selection follow this record.",
                        self.grid.model().rows()[index].id + 1
                    )
                }
                Action::CursorMoved { row, column } => {
                    self.status = format!(
                        "Cursor on record #{:06}, column {}.",
                        self.grid.model().rows()[row].id + 1,
                        column + 1
                    )
                }
                Action::CopyRequested(target) => {
                    self.status = format!(
                        "Copy requested: {}",
                        self.grid.copy_text(target).unwrap_or_default()
                    );
                }
                Action::RowsSelected => {
                    self.status = format!(
                        "{} rows marked for bulk actions.",
                        self.grid.model().selected_indices().count()
                    );
                }
                Action::FilterChanged => {
                    self.status = format!(
                        "Search: {} · {} matching records.",
                        self.grid.search_query(),
                        self.grid.model().visible_len()
                    );
                }
                Action::RowActivated(index) => {
                    self.status = format!(
                        "Opened record #{:06}: {}",
                        self.grid.model().rows()[index].id + 1,
                        self.grid.model().rows()[index].name()
                    )
                }
                Action::ColumnResized { column, width } => {
                    self.status = format!("Column {} resized to {width} cells.", column + 1)
                }
                Action::HeaderFocused(column) => {
                    self.status = format!(
                        "Header {} focused. Enter sorts; +/- resizes; Escape leaves headers.",
                        column + 1
                    )
                }
                _ => (),
            }
        }
        false
    }
    pub(super) fn tick(&mut self) {
        self.tick_at(Instant::now());
        if self.live && self.last_feed.elapsed() >= Duration::from_secs(1) {
            self.tweens.clear();
            self.generation += 1;
            if self.count == 0 {
                self.count = 8;
            }
            let start = Instant::now();
            if self.scenario == Scenario::Paged {
                self.grid.reload_page();
                fulfill_page(&mut self.grid, self.generation, self.count);
                self.flash_visible_changes();
                self.status = format!(
                    "Refreshed page {}: {} records resident.",
                    self.grid.page_state().unwrap().page + 1,
                    self.grid.model().rows().len()
                );
                self.last_feed = Instant::now();
                return;
            }
            self.grid
                .replace_rows(records(self.scenario, self.count, self.generation));
            self.flash_visible_changes();
            self.status = format!(
                "Feed update {}: {} records replaced in {:.2} ms; sort and selection kept by ID.",
                self.generation,
                grouped(self.count),
                start.elapsed().as_secs_f64() * 1000.0
            );
            self.last_feed = Instant::now();
        }
    }
    pub(super) fn flash_visible_changes(&mut self) {
        if !self.motion {
            return;
        }
        let page_start = self
            .grid
            .page_state()
            .filter(|p| p.mode == ratagrid::PaginationMode::Client)
            .map_or(0, |p| p.page * p.page_size.get());
        let start = page_start + self.grid.row_offset();
        let end = self
            .grid
            .page_state()
            .filter(|p| p.mode == ratagrid::PaginationMode::Client)
            .map_or(start.saturating_add(6), |p| {
                (start + 6).min(page_start + p.page_size.get())
            });
        for position in start..end {
            if let Some(index) = self.grid.model().index_at(position) {
                for column in [3, 4, 5] {
                    self.grid.flash_cell(index, column, FLASH_TIME);
                }
            }
        }
    }
    pub(super) fn boost(&mut self, now: Instant) {
        self.tick_at(now);
        let page_start = self
            .grid
            .page_state()
            .filter(|p| p.mode == ratagrid::PaginationMode::Client)
            .map_or(0, |p| p.page * p.page_size.get());
        let page_end = self
            .grid
            .page_state()
            .filter(|p| p.mode == ratagrid::PaginationMode::Client)
            .map_or(self.grid.model().rows().len(), |p| {
                (page_start + p.page_size.get()).min(p.total_rows.unwrap())
            });
        let selected = self
            .grid
            .model()
            .selected_position()
            .filter(|position| (page_start..page_end).contains(position))
            .and_then(|_| self.grid.model().selected_index());
        let Some(index) = selected.or_else(|| {
            self.grid
                .model()
                .index_at(page_start + self.grid.row_offset())
        }) else {
            self.status = "No row to boost. Choose a dataset or enable Live.".into();
            return;
        };
        let row = &self.grid.model().rows()[index];
        let start = Values::from_record(row);
        let end = Values {
            latency: start.latency.saturating_add(1250),
            delta: start.delta.saturating_add(20),
            memory: start.memory.saturating_add(256),
        };
        let id = row.id;
        self.tweens.retain(|tween| tween.index != index);
        if self.motion {
            self.tweens.push(RowTween {
                index,
                id,
                start,
                end,
                started: now,
            });
            for column in [3, 4, 5] {
                self.grid.flash_cell(index, column, FLASH_TIME);
            }
            self.status = format!(
                "Boosting record #{:06}: Latency, Delta and Memory animate together.",
                id + 1
            );
        } else {
            self.grid.update_row(index, |row| end.write(row));
            self.status = format!(
                "Updated record #{:06}: Latency +1250 ms, Delta +20, Memory +256 MiB.",
                id + 1
            );
        }
    }
    pub(super) fn tick_at(&mut self, now: Instant) {
        self.grid
            .advance_animations(now.saturating_duration_since(self.last_tick));
        self.last_tick = now;
        self.tweens.retain(|tween| {
            if self
                .grid
                .model()
                .rows()
                .get(tween.index)
                .is_none_or(|row| row.id != tween.id)
            {
                return false;
            }
            let progress = (now.saturating_duration_since(tween.started).as_secs_f64()
                / TWEEN_TIME.as_secs_f64())
            .min(1.0);
            let eased = 1.0 - (1.0 - progress).powi(3);
            let value = tween.start.interpolate(tween.end, eased);
            if Values::from_record(&self.grid.model().rows()[tween.index]) != value {
                self.grid.update_row(tween.index, |row| value.write(row));
            }
            if progress == 1.0 {
                self.status = format!(
                    "Updated record #{:06}: Latency +1250 ms, Delta +20, Memory +256 MiB.",
                    tween.id + 1
                );
                false
            } else {
                true
            }
        });
    }
}

pub(super) fn grouped(value: usize) -> String {
    let digits = value.to_string();
    let mut result = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            result.push(',');
        }
        result.push(c);
    }
    result
}

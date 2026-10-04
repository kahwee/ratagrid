//! Try: cargo run --example explorer
//! A small source simulation deliberately exposes pending/error states. The
//! application owns fetching, clipboard handling, and column-control bindings.
mod support;

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratagrid::{Action, Column, Grid, LoadState, PageRequest, SortDirection};
use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Stylize},
    widgets::Paragraph,
};
use std::{
    io,
    num::NonZeroUsize,
    time::{Duration, Instant},
};

#[derive(Clone)]
struct Job {
    id: usize,
    name: String,
    status: &'static str,
    owner: &'static str,
}
fn records() -> Vec<Job> {
    let names = ["Compile", "Test", "Deploy", "Index 東京", "Lint", "Package"];
    (0..120)
        .map(|id| Job {
            id,
            name: format!("{} #{:03}", names[id % 6], id + 1),
            status: ["Running", "Queued", "Complete", "Blocked"][id % 4],
            owner: ["Ada", "Lin", "Sam", "Noor"][id % 4],
        })
        .collect()
}
fn columns() -> Vec<Column<Job>> {
    vec![
        Column::new("ID", 8, |r: &Job| (r.id + 1).to_string()).sortable(|a, b| a.id.cmp(&b.id)),
        Column::new("Job", 26, |r: &Job| r.name.clone()).sortable(|a, b| a.name.cmp(&b.name)),
        Column::new("Status", 16, |r: &Job| r.status.into())
            .sortable(|a, b| a.status.cmp(b.status)),
        Column::new("Owner", 14, |r: &Job| r.owner.into()).sortable(|a, b| a.owner.cmp(b.owner)),
    ]
}
struct Pending {
    request: PageRequest,
    query: String,
    deadline: Instant,
    fail: bool,
}
struct App {
    grid: Grid<Job>,
    remote: bool,
    pending: Option<Pending>,
    status: String,
    generation: usize,
}
impl App {
    fn new(remote: bool) -> Self {
        let mut grid = if remote {
            Grid::new_paged(columns(), None, NonZeroUsize::new(12).unwrap())
        } else {
            Grid::new(columns(), records()).with_pagination(NonZeroUsize::new(12).unwrap())
        }
        .with_row_id(|r| r.id);
        grid.set_column_pinned(0, true);
        let mut app = Self {
            grid,
            remote,
            pending: None,
            status: "Ready. ID is pinned; F2 switches data source.".into(),
            generation: 0,
        };
        if remote {
            app.fetch(false);
        }
        app
    }
    fn fetch(&mut self, fail: bool) {
        self.pending = Some(Pending {
            request: self.grid.page_request().unwrap(),
            query: self.grid.search_query().into(),
            deadline: Instant::now() + Duration::from_millis(350),
            fail,
        });
        self.status = "Fetching records…".into();
    }
    fn tick(&mut self) {
        if !self
            .pending
            .as_ref()
            .is_some_and(|p| Instant::now() >= p.deadline)
        {
            return;
        }
        let pending = self.pending.take().unwrap();
        if pending.fail {
            let _ = self
                .grid
                .set_page_error(pending.request, "Simulated source failure");
            self.status = "Request failed. Click Retry or press F5.".into();
            return;
        }
        let query = pending.query.to_lowercase();
        let mut records: Vec<_> = records()
            .into_iter()
            .filter(|r| {
                query.is_empty()
                    || format!("{} {} {} {}", r.id + 1, r.name, r.status, r.owner)
                        .to_lowercase()
                        .contains(&query)
            })
            .collect();
        if let Some(sort) = pending.request.sort {
            records.sort_by(|a, b| {
                let ordering = match sort.column {
                    0 => a.id.cmp(&b.id),
                    1 => a.name.cmp(&b.name),
                    2 => a.status.cmp(b.status),
                    _ => a.owner.cmp(b.owner),
                };
                match sort.direction {
                    SortDirection::Ascending => ordering,
                    SortDirection::Descending => ordering.reverse(),
                }
            });
        }
        let page = records
            .into_iter()
            .skip(pending.request.offset())
            .take(pending.request.page_size.get())
            .collect();
        if self.grid.set_page_data(pending.request, page).is_ok() {
            self.status = format!(
                "Loaded page {} for query {:?}.",
                pending.request.page + 1,
                pending.query
            );
        }
    }
    fn handle(&mut self, event: &Event) -> bool {
        if !self.grid.is_searching()
            && let Event::Key(key) = event
            && key.kind != KeyEventKind::Release
        {
            if key.modifiers == KeyModifiers::NONE {
                match key.code {
                    KeyCode::Char('q') => return true,
                    KeyCode::F(2) => {
                        *self = Self::new(!self.remote);
                        return false;
                    }
                    KeyCode::F(3) if self.remote => {
                        self.grid.reload_page();
                        self.fetch(true);
                        return false;
                    }
                    KeyCode::F(4) if !self.remote => {
                        self.generation += 1;
                        let mut rows = records();
                        let count = rows.len();
                        rows.rotate_left(self.generation % count);
                        self.grid.replace_rows(rows);
                        self.status =
                            "Refreshed with reordered records; cursor and marked rows follow IDs."
                                .into();
                        return false;
                    }
                    _ => (),
                }
            }
            if key.modifiers == KeyModifiers::CONTROL {
                match key.code {
                    KeyCode::Char('h') => {
                        if let Some((_, column)) = self.grid.cursor() {
                            self.grid.set_column_visible(column, false);
                        }
                        self.status = "Column hidden. Ctrl+R restores the layout.".into();
                        return false;
                    }
                    KeyCode::Char('p') => {
                        if let Some((_, column)) = self.grid.cursor() {
                            let pinned = self.grid.is_column_pinned(column);
                            self.grid.set_column_pinned(column, !pinned);
                        }
                        self.status = "Column pin toggled.".into();
                        return false;
                    }
                    KeyCode::Char('r') => {
                        for column in 0..4 {
                            self.grid.set_column_visible(column, true);
                            self.grid.set_column_pinned(column, column == 0);
                        }
                        self.grid.set_column_order(vec![0, 1, 2, 3]).unwrap();
                        self.status = "Column layout restored.".into();
                        return false;
                    }
                    KeyCode::Left | KeyCode::Right => {
                        if let Some((_, column)) = self.grid.cursor() {
                            let mut order = self.grid.column_order().to_vec();
                            let position = order.iter().position(|&c| c == column).unwrap();
                            let next = position
                                .saturating_add_signed(if key.code == KeyCode::Left {
                                    -1
                                } else {
                                    1
                                })
                                .min(order.len() - 1);
                            order.swap(position, next);
                            self.grid.set_column_order(order).unwrap();
                        }
                        self.status = "Column reordered; pinned columns remain first.".into();
                        return false;
                    }
                    _ => (),
                }
            }
        }
        if let Some(action) = self.grid.handle_event(event) {
            match action {
                Action::PageRequested(_) => self.fetch(false),
                Action::CopyRequested(target) => {
                    // An actual application writes this text to its clipboard backend.
                    // This demo shows the payload so it runs on every terminal platform.
                    let text = self.grid.copy_text(target).unwrap_or_default();
                    self.status = format!(
                        "Copy payload: {}",
                        text.replace('\t', " | ").replace('\n', " / ")
                    );
                }
                Action::RowsSelected => {
                    self.status = format!(
                        "{} rows marked.",
                        self.grid.model().selected_indices().count()
                    )
                }
                Action::FilterChanged => {
                    self.status = format!("{} matching records.", self.grid.model().visible_len())
                }
                Action::RowActivated(row) => {
                    self.status =
                        format!("Activated job #{}.", self.grid.model().rows()[row].id + 1)
                }
                _ => (),
            }
        }
        false
    }
}
fn main() -> io::Result<()> {
    support::run(true, |terminal| {
        let mut app = App::new(false);
        loop {
            app.tick();
            terminal.draw(|frame| {
                let sections=Layout::vertical([Constraint::Length(2),Constraint::Min(1),Constraint::Length(2),Constraint::Length(4)]).split(frame.area());
                let title=format!("Ratagrid explorer · {} · {} marked · {}",if app.remote{"remote source"}else{"owned records"},app.grid.model().selected_indices().count(), match app.grid.load_state(){LoadState::Loading=>"loading",LoadState::Error(_)=>"error",LoadState::Empty=>"empty",LoadState::Ready=>"ready"});
                frame.render_widget(Paragraph::new(title).bold().fg(Color::Cyan),sections[0]);
                frame.render_widget(app.grid.widget(),sections[1]);
                frame.render_widget(Paragraph::new(app.status.as_str()),sections[2]);
                frame.render_widget(Paragraph::new("/: search · Space: mark row · Shift+arrows/click: range · Ctrl+A: mark page\nCtrl+C: copy cell · Ctrl+Shift+C: copy rows · Esc: clear marks\nCtrl+H: hide · Ctrl+P: pin · Ctrl+←/→: reorder · Ctrl+R: restore columns\nF2: owned/remote · F3: fail remote load · F4: reorder refresh · F5: retry · q: quit"),sections[3]);
            })?;
            if event::poll(Duration::from_millis(40))? && app.handle(&event::read()?) {
                break;
            }
        }
        Ok(())
    })
}

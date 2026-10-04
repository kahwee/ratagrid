//! Scenarios, resident records and the simulated indexed page source.
use ratagrid::{Column, Grid, PageRequest, SortDirection};
use ratatui::style::{Color, Modifier, Style};
use std::{cell::Cell, num::NonZeroUsize, rc::Rc};

pub(super) const SCENARIOS: [Scenario; 7] = [
    Scenario::Jobs,
    Scenario::HundredK,
    Scenario::Million,
    Scenario::Unicode,
    Scenario::Wide,
    Scenario::Empty,
    Scenario::Paged,
];

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Scenario {
    Jobs,
    HundredK,
    Million,
    Unicode,
    Wide,
    Empty,
    Paged,
}
impl Scenario {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Jobs => "Jobs",
            Self::HundredK => "100K rows",
            Self::Million => "1M rows",
            Self::Unicode => "Unicode",
            Self::Wide => "32 columns",
            Self::Empty => "Empty",
            Self::Paged => "Paged 100M",
        }
    }
    pub(super) fn slug(self) -> &'static str {
        match self {
            Self::Jobs => "jobs",
            Self::HundredK => "100k",
            Self::Million => "million",
            Self::Unicode => "unicode",
            Self::Wide => "wide",
            Self::Empty => "empty",
            Self::Paged => "paged",
        }
    }
    pub(super) fn compact_label(self) -> &'static str {
        match self {
            Self::Jobs => "Jobs",
            Self::HundredK => "100K",
            Self::Million => "1M",
            Self::Unicode => "UTF8",
            Self::Wide => "Wide",
            Self::Empty => "∅",
            Self::Paged => "Paged",
        }
    }
    pub(super) fn rows(self) -> usize {
        match self {
            Self::Jobs => 250,
            Self::HundredK => 100_000,
            Self::Million => 1_000_000,
            Self::Unicode => 128,
            Self::Wide => 10_000,
            Self::Empty => 0,
            Self::Paged => 100_000_000,
        }
    }
    pub(super) fn columns(self) -> usize {
        if self == Self::Wide { 32 } else { 9 }
    }
    pub(super) fn description(self) -> &'static str {
        match self {
            Self::Paged => {
                "100 million virtual records, 50 loaded. Click pager buttons; [ and ] change pages; P changes page size."
            }
            Self::Jobs => {
                "Select a row, then click Boost below: other cells count up and glow. A toggles motion."
            }
            Self::HundredK => "100,000 records. Try sorting latency, then End and Home.",
            Self::Million => {
                "One million records. Only visible cells are formatted; sorting visits all rows."
            }
            Self::Unicode => {
                "CJK, combining accents, emoji, long paths, blank cells and control characters."
            }
            Self::Wide => {
                "32 columns. Arrows move the cursor; Shift+wheel scrolls; Tab reveals each header."
            }
            Self::Empty => {
                "No records. Headers still sort and resize. Turn on the live feed to add rows."
            }
        }
    }
}

#[derive(Clone)]
pub(super) struct Record {
    pub(super) id: usize,
    pub(super) latency: Option<u64>,
    pub(super) delta: i64,
    pub(super) memory: u64,
    pub(super) unicode: bool,
    pub(super) remote: bool,
}
impl Record {
    pub(super) fn label(&self) -> &'static str {
        if self.remote {
            return "Indexed record";
        }
        let labels = if self.unicode {
            [
                "東京 / build",
                "cafe\u{301} / tests",
                "🦀 Rust / lint",
                "👩🏽‍💻 agent / review",
                "🚀 deploy / preview",
                "a\u{1b}\n\tb / controls",
                "very/long/path/that/should/be/clipped/without/breaking/the/next/column",
                "",
            ]
        } else {
            [
                "Compile workspace",
                "Run integration tests",
                "Lint source",
                "Review pull request",
                "Deploy preview",
                "Build documentation",
                "Sync fixtures",
                "Package release",
            ]
        };
        labels[self.id % labels.len()]
    }
    pub(super) fn name(&self) -> String {
        format!("{} #{:06}", self.label(), self.id + 1)
    }
    pub(super) fn state(&self) -> &'static str {
        ["running", "queued", "complete", "blocked"][self.id % 4]
    }
    pub(super) fn owner(&self) -> &'static str {
        ["Ada", "Lin", "Sam", "Noor"][self.id % 4]
    }
    pub(super) fn region(&self) -> &'static str {
        ["us-east", "eu-west", "ap-south", "local"][self.id % 4]
    }
}
pub(super) fn mixed(mut n: u64) -> u64 {
    n = (n ^ (n >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    n = (n ^ (n >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    n ^ (n >> 31)
}
pub(super) fn records(scenario: Scenario, count: usize, generation: u64) -> Vec<Record> {
    (0..count)
        .map(|id| {
            let value = mixed(id as u64 + generation * 7919 + 1);
            Record {
                id,
                latency: (!id.is_multiple_of(11)).then_some(value % 20_000),
                delta: (value % 201) as i64 - 100,
                memory: value % 8192,
                unicode: scenario == Scenario::Unicode,
                remote: false,
            }
        })
        .collect()
}
pub(super) fn columns(count: usize) -> Vec<Column<Record>> {
    themed_columns(count, Rc::new(Cell::new(false)))
}

fn themed_columns(count: usize, light: Rc<Cell<bool>>) -> Vec<Column<Record>> {
    let mut columns = vec![
        Column::new("ID", 9, |r: &Record| format!("{:06}", r.id + 1))
            .sortable(|a, b| a.id.cmp(&b.id)),
        Column::new("Workload", 29, Record::name)
            .sortable(|a, b| a.label().cmp(b.label()).then(a.id.cmp(&b.id))),
        Column::new("State", 12, |r: &Record| r.state().into())
            .sortable(|a, b| a.state().cmp(b.state())),
        Column::new("Latency", 14, |r: &Record| {
            r.latency.map_or("—".into(), |n| format!("{n:>6} ms"))
        })
        .sortable(|a, b| a.latency.cmp(&b.latency)),
        Column::new("Delta", 11, |r: &Record| format!("{:+} %", r.delta))
            .sortable(|a, b| a.delta.cmp(&b.delta)),
        Column::new("Memory", 13, |r: &Record| format!("{} MiB", r.memory))
            .sortable(|a, b| a.memory.cmp(&b.memory)),
        Column::new("Owner", 12, |r: &Record| r.owner().into())
            .sortable(|a, b| a.owner().cmp(b.owner())),
        Column::new("Region", 12, |r: &Record| r.region().into())
            .sortable(|a, b| a.region().cmp(b.region())),
        Column::new("Notes (display only)", 34, |r: &Record| {
            if r.unicode {
                "東京 · cafe\u{301} · 🦀 · long text".into()
            } else {
                "Click headers to sort actual values".into()
            }
        }),
    ];
    for index in 9..count {
        columns.push(
            Column::new(format!("Metric {:02}", index - 8), 15, move |r: &Record| {
                format!("{:>10}", mixed(r.id as u64 + index as u64) % 1_000_000)
            })
            .sortable(move |a, b| {
                mixed(a.id as u64 + index as u64).cmp(&mixed(b.id as u64 + index as u64))
            }),
        );
    }
    columns.truncate(count);
    columns
        .into_iter()
        .enumerate()
        .map(|(index, column)| {
            let palette = light.clone();
            // Foreground only: selection, hover and animation keep their background.
            column.cell_style(move |r| {
                let light = palette.get();
                let color = |dark, pale| if light { pale } else { dark };
                let cyan = color(Color::Rgb(119, 207, 238), Color::Rgb(0, 99, 135));
                let green = color(Color::Rgb(105, 231, 193), Color::Rgb(0, 112, 79));
                let amber = color(Color::Rgb(255, 205, 112), Color::Rgb(151, 88, 0));
                let red = color(Color::Rgb(255, 131, 150), Color::Rgb(177, 38, 66));
                let purple = color(Color::Rgb(192, 164, 255), Color::Rgb(109, 66, 175));
                let muted = color(Color::Rgb(128, 149, 170), Color::Rgb(82, 106, 125));
                let fg = match index {
                    0 => muted,
                    1 => cyan,
                    2 => match r.state() {
                        "complete" => green,
                        "running" => cyan,
                        "queued" => amber,
                        _ => red,
                    },
                    3 => match r.latency {
                        None => muted,
                        Some(n) if n >= 15_000 => red,
                        Some(n) if n >= 5_000 => amber,
                        _ => green,
                    },
                    4 if r.delta < 0 => red,
                    4 => green,
                    5 | 6 => purple,
                    7 => cyan,
                    _ => muted,
                };
                let style = Style::default().fg(fg);
                if index == 2 {
                    style.add_modifier(Modifier::BOLD)
                } else {
                    style
                }
            })
        })
        .collect()
}

pub(super) fn page_records(request: PageRequest, total: usize, generation: u64) -> Vec<Record> {
    let end = request
        .offset()
        .saturating_add(request.page_size.get())
        .min(total);
    let descending = request
        .sort
        .is_some_and(|s| s.direction == SortDirection::Descending);
    let groups = match request.sort.map(|s| s.column) {
        Some(2) => Some([3_usize, 2, 1, 0]), // blocked, complete, queued, running
        Some(6) => Some([0, 1, 3, 2]),       // Ada, Lin, Noor, Sam
        Some(7) => Some([2, 1, 3, 0]),       // ap-south, eu-west, local, us-east
        _ => None,
    };
    (request.offset()..end)
        .map(|rank| {
            let id = if let Some(mut groups) = groups {
                if descending {
                    groups.reverse();
                }
                let mut local = rank;
                let mut found = 0;
                for group in groups {
                    let count = total / 4 + usize::from(group < total % 4);
                    if local < count {
                        found = group + local * 4;
                        break;
                    }
                    local -= count;
                }
                found
            } else if descending {
                total - 1 - rank
            } else {
                rank
            };
            Record {
                id,
                latency: Some(id as u64 + generation),
                delta: id as i64,
                memory: id as u64 + generation,
                unicode: false,
                remote: true,
            }
        })
        .collect()
}
pub(super) fn fulfill_page(grid: &mut Grid<Record>, generation: u64, source_total: usize) {
    // The virtual source has no text index. Demonstrate an indexed exact-ID query
    // rather than scanning 100 million synthetic records for every search.
    let query = grid.search_query().to_owned();
    if !query.is_empty() {
        let id = query
            .strip_prefix("id:")
            .and_then(|text| text.trim().parse::<usize>().ok());
        let Some(id) = id.filter(|&id| id > 0 && id <= source_total) else {
            let request = grid.page_request().expect("external source");
            grid.set_page_error(
                request,
                "Use id:NUMBER for this virtual source; explorer demonstrates full text search",
            )
            .expect("current request");
            return;
        };
        grid.set_total_rows(1);
        let request = grid.page_request().expect("external source");
        grid.set_page_data(
            request,
            vec![Record {
                id: id - 1,
                latency: Some((id - 1) as u64 + generation),
                delta: (id - 1) as i64,
                memory: (id - 1) as u64 + generation,
                unicode: false,
                remote: true,
            }],
        )
        .expect("one matching record");
    } else {
        grid.set_total_rows(source_total);
        let request = grid.page_request().expect("external source");
        grid.set_page_data(request, page_records(request, source_total, generation))
            .expect("complete current page");
    }
}
pub(super) fn make_grid(
    scenario: Scenario,
    count: usize,
    column_count: usize,
    generation: u64,
    light: Rc<Cell<bool>>,
) -> Grid<Record> {
    if scenario != Scenario::Paged {
        return Grid::new(
            themed_columns(column_count, light),
            records(scenario, count, generation),
        )
        .with_row_id(|r| r.id);
    }
    let mut cols = themed_columns(column_count, light);
    cols[0] = Column::new("ID", 12, |r: &Record| format!("{:09}", r.id + 1))
        .sortable(|a, b| a.id.cmp(&b.id));
    let mut grid =
        Grid::new_paged(cols, count, NonZeroUsize::new(50).unwrap()).with_row_id(|r| r.id);
    fulfill_page(&mut grid, generation, count);
    grid
}

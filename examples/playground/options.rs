//! Playground command-line options.
use super::data::{SCENARIOS, Scenario};
use std::path::PathBuf;

pub(super) struct Options {
    pub(super) scenario: Scenario,
    pub(super) rows: Option<usize>,
    pub(super) columns: Option<usize>,
    pub(super) snapshot: Option<PathBuf>,
    pub(super) benchmark: bool,
    pub(super) stress: bool,
    pub(super) stress_large: bool,
    pub(super) width: u16,
    pub(super) height: u16,
    pub(super) animation_frame: Option<u64>,
}
impl Options {
    pub(super) fn parse() -> Result<Option<Self>, String> {
        let mut options = Self {
            scenario: Scenario::Jobs,
            rows: None,
            columns: None,
            snapshot: None,
            benchmark: false,
            stress: false,
            stress_large: false,
            width: 132,
            height: 34,
            animation_frame: None,
        };
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--help" | "-h" => {
                    println!(
                        "Ratagrid playground\n\n  --scenario jobs|100k|million|unicode|wide|empty|paged\n  --rows N           Override row count (0..=2000000)\n  --columns N        Override columns (1..=128)\n  --snapshot FILE    Render the initial screen as an SVG, without a terminal\n  --width N          Snapshot/benchmark terminal width (default 132)\n  --height N         Snapshot/benchmark terminal height (default 34)\n  --animation-frame MS  Snapshot Boost at an elapsed animation time\n  --benchmark        Measure all owned-data scenarios; use --release\n  --stress           Fixed stress suite: percentiles, sorting, updates, paging\n  --stress-large     Include 10 million resident rows (use --release)\n\nKeys: 1..7 scenarios · B boost row · A motion · P pagination/page size · [ and ] previous/next · L live feed · T theme · R reset · Q quit\nMouse: click scenario tabs and footer controls; sort, select, scroll; hold a header separator and drag left/right to resize."
                    );
                    return Ok(None);
                }
                "--benchmark" => options.benchmark = true,
                "--stress" => options.stress = true,
                "--stress-large" => {
                    options.stress = true;
                    options.stress_large = true;
                }
                "--scenario" => {
                    let value = args.next().ok_or("--scenario requires a value")?;
                    options.scenario = *SCENARIOS
                        .iter()
                        .find(|s| s.slug() == value)
                        .ok_or(format!("Unknown scenario: {value}"))?;
                }
                "--snapshot" => {
                    options.snapshot = Some(args.next().ok_or("--snapshot requires a path")?.into())
                }
                "--animation-frame" => {
                    options.animation_frame = Some(
                        args.next()
                            .ok_or("--animation-frame requires milliseconds")?
                            .parse::<u64>()
                            .map_err(|_| "Invalid animation milliseconds")?
                            .min(5000),
                    );
                }
                "--rows" | "--columns" | "--width" | "--height" => {
                    let value = args
                        .next()
                        .ok_or(format!("{arg} requires a number"))?
                        .parse::<usize>()
                        .map_err(|_| format!("Invalid number for {arg}"))?;
                    match arg.as_str() {
                        "--rows" if value <= 2_000_000 => options.rows = Some(value),
                        "--columns" if (1..=128).contains(&value) => options.columns = Some(value),
                        "--width" if (1..=500).contains(&value) => options.width = value as u16,
                        "--height" if (1..=200).contains(&value) => options.height = value as u16,
                        _ => return Err(format!("Out-of-range value for {arg}")),
                    }
                }
                _ => return Err(format!("Unknown argument: {arg}; use --help")),
            }
        }
        if options.scenario == Scenario::Paged && options.columns.is_some_and(|n| n > 9) {
            return Err("The indexed paged scenario supports up to 9 columns".into());
        }
        if options.animation_frame.is_some() && options.snapshot.is_none() {
            return Err("--animation-frame requires --snapshot".into());
        }
        Ok(Some(options))
    }
}

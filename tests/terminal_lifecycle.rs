//! Isolated real-terminal probe driven by scripts/test_terminal_cleanup.py.
#![cfg(unix)]

#[path = "../examples/support/mod.rs"]
mod terminal_support;

use std::{io, panic};

use crossterm::terminal::is_raw_mode_enabled;
use ratatui::widgets::Paragraph;

#[test]
#[ignore = "requires an isolated PTY; run python3 scripts/test_terminal_cleanup.py"]
fn terminal_cleanup_probe() {
    let mode = std::env::var("RATAGRID_CLEANUP_PROBE").expect("PTY runner supplies the case");
    let (outcome, capture) = mode.split_once(':').expect("outcome:capture");
    let mouse = capture == "mouse";
    assert!(!is_raw_mode_enabled().unwrap());
    let result = panic::catch_unwind(|| {
        terminal_support::run(mouse, |terminal| {
            assert!(is_raw_mode_enabled()?);
            terminal.draw(|frame| {
                frame.render_widget(Paragraph::new("terminal cleanup probe"), frame.area());
            })?;
            match outcome {
                "success" => Ok(()),
                "error" => Err(io::Error::other("original application error")),
                "panic" | "panic-without-hook" => {
                    if outcome == "panic-without-hook" {
                        // Ratatui's hook restores raw mode itself. Remove it in
                        // this isolated child to prove the guard also restores it.
                        panic::set_hook(Box::new(|_| {}));
                    }
                    panic!("intentional application panic");
                }
                other => panic!("unknown probe outcome: {other}"),
            }
        })
    });
    match outcome {
        "success" => result.unwrap().unwrap(),
        "error" => {
            let error = result.unwrap().unwrap_err();
            assert_eq!(error.kind(), io::ErrorKind::Other);
            assert_eq!(error.to_string(), "original application error");
        }
        "panic" | "panic-without-hook" => {
            let panic = result.expect_err("application panic must propagate");
            assert_eq!(
                panic.downcast_ref::<&str>(),
                Some(&"intentional application panic")
            );
        }
        _ => unreachable!(),
    }
    assert!(!is_raw_mode_enabled().unwrap());
    println!("PROBE_FINISHED {mode}");
}

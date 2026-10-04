//! Terminal lifecycle shared by the larger demos; not part of Ratagrid's API.
use std::io;

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
};
use ratatui::DefaultTerminal;

struct Session {
    terminal: DefaultTerminal,
    mouse: bool,
}

impl Drop for Session {
    fn drop(&mut self) {
        if self.mouse {
            let _ = execute!(io::stdout(), DisableMouseCapture);
        }
        ratatui::restore();
    }
}

/// Restore the terminal on success, I/O errors and unwinding. Ratatui also
/// installs its default panic hook. Keep mouse capture optional for keyboard demos.
pub fn run(
    mouse: bool,
    app: impl FnOnce(&mut DefaultTerminal) -> io::Result<()>,
) -> io::Result<()> {
    let mut session = Session {
        terminal: ratatui::init(),
        mouse,
    };
    if mouse {
        execute!(io::stdout(), EnableMouseCapture)?;
    }
    let result = app(&mut session.terminal);
    let cleanup = if mouse {
        execute!(io::stdout(), DisableMouseCapture)
    } else {
        Ok(())
    };
    session.mouse = false;
    result.and(cleanup)
}

use std::{
    io,
    sync::Once,
    sync::atomic::{AtomicBool, Ordering},
};

use anyhow::{Context, Result};
use crossterm::{
    cursor::{Hide, Show},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

static TERMINAL_ACTIVE: AtomicBool = AtomicBool::new(false);
static PANIC_HOOK: Once = Once::new();

pub struct TerminalGuard {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
}

impl TerminalGuard {
    pub fn enter() -> Result<Self> {
        Self::install_panic_hook();
        enable_raw_mode().context("Не удалось включить raw mode")?;
        TERMINAL_ACTIVE.store(true, Ordering::SeqCst);

        let mut stdout = io::stdout();
        if let Err(error) = execute!(stdout, EnterAlternateScreen, Hide) {
            TERMINAL_ACTIVE.store(false, Ordering::SeqCst);
            restore_terminal();
            return Err(error.into());
        }

        let mut terminal = match Terminal::new(CrosstermBackend::new(stdout)) {
            Ok(terminal) => terminal,
            Err(error) => {
                TERMINAL_ACTIVE.store(false, Ordering::SeqCst);
                restore_terminal();
                return Err(error.into());
            }
        };

        if let Err(error) = terminal.hide_cursor() {
            TERMINAL_ACTIVE.store(false, Ordering::SeqCst);
            restore_terminal();
            return Err(error.into());
        }

        Ok(Self { terminal })
    }

    pub fn terminal_mut(&mut self) -> &mut Terminal<CrosstermBackend<io::Stdout>> {
        &mut self.terminal
    }

    fn install_panic_hook() {
        PANIC_HOOK.call_once(|| {
            let previous_hook = std::panic::take_hook();
            std::panic::set_hook(Box::new(move |panic| {
                if TERMINAL_ACTIVE.swap(false, Ordering::SeqCst) {
                    restore_terminal();
                }
                previous_hook(panic);
            }));
        });
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        if TERMINAL_ACTIVE.swap(false, Ordering::SeqCst) {
            restore_terminal();
        }
    }
}

fn restore_terminal() {
    let mut stdout = io::stdout();
    let _ = execute!(stdout, Show, LeaveAlternateScreen);
    let _ = disable_raw_mode();
}

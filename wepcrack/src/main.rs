use crossterm::{
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{prelude::CrosstermBackend, Terminal};
use std::{
    error::Error,
    sync::atomic::{self, AtomicBool},
};

pub mod app;
pub mod ieee80211;
pub mod keycracker;
pub mod nl80211;
pub mod rc4;
pub mod ui;
pub mod util;
pub mod wep;

static TERMINAL_LOCK: AtomicBool = AtomicBool::new(true);

fn main() -> Result<(), Box<dyn Error>> {
    //Create the app
    let mut app = app::App::create()?;

    //Initialize the terminal
    crossterm::terminal::enable_raw_mode()?;
    std::io::stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stdout()))?;

    //Install the panic hook
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic| {
        TERMINAL_LOCK.store(false, atomic::Ordering::SeqCst);
        crossterm::terminal::disable_raw_mode().unwrap();
        std::io::stdout().execute(LeaveAlternateScreen).unwrap();

        original_hook(panic);
    }));

    //Run the main UI loop
    app.run(&mut terminal)?;

    //Clean up the terminal
    if TERMINAL_LOCK.load(atomic::Ordering::SeqCst) {
        crossterm::terminal::disable_raw_mode().unwrap();
        std::io::stdout().execute(LeaveAlternateScreen).unwrap();
    }

    Ok(())
}

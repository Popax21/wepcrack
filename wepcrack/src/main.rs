use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{prelude::CrosstermBackend, Terminal};
use std::{
    error::Error,
    sync::{atomic, Arc},
    time::Duration,
};
use ui::UIScene;

pub mod keycrack;
pub mod rc4;

pub mod ui;

fn main() -> Result<(), Box<dyn Error>> {
    //Initialize the terminal
    crossterm::terminal::enable_raw_mode()?;
    std::io::stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stdout()))?;

    //Run the main UI loop
    let mut app = App {
        scene: Box::from(ui::keycrack::UIKeycrack {}),
    };
    app.run(&mut terminal)?;

    //Cleanup the terminal
    crossterm::terminal::disable_raw_mode()?;
    std::io::stdout().execute(LeaveAlternateScreen)?;

    Ok(())
}

struct App {
    scene: Box<dyn UIScene>,
}

impl App {
    fn run(
        self: &mut App,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    ) -> Result<(), Box<dyn Error>> {
        //Install a Ctrl+C handler
        let should_quit = Arc::new(atomic::AtomicBool::new(false));
        {
            let should_quit = should_quit.clone();
            ctrlc::set_handler(move || should_quit.store(true, atomic::Ordering::SeqCst))?;
        }

        //Run the main UI loop
        while !should_quit.load(atomic::Ordering::SeqCst) {
            //Draw the current UI scene
            terminal.draw(|f| self.scene.draw_ui(f))?;

            //Poll for events
            if event::poll(Duration::from_millis(16))? {
                while event::poll(Duration::from_millis(0))? {
                    let evt = event::read()?;

                    //Quit on Esc
                    if let Event::Key(key) = &evt {
                        if key.code == KeyCode::Esc {
                            return Ok(());
                        }
                    }

                    //Let the scene handle it
                    self.scene.handle_event(&evt);
                }
            }
        }

        Ok(())
    }
}

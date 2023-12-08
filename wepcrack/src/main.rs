use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    prelude::{Alignment, Constraint, CrosstermBackend, Direction, Layout},
    style::Stylize,
    text::Line,
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};
use std::{
    error::Error,
    sync::{
        atomic::{self, AtomicBool},
        Arc,
    },
    time::Duration,
};
use ui::UIScene;

pub mod keycracker;
pub mod nl80211;
pub mod rc4;
pub mod ui;
pub mod util;
pub mod wep;

static TERMINAL_LOCK: AtomicBool = AtomicBool::new(true);

fn main() -> Result<(), Box<dyn Error>> {
    //Initialize the app
    let mut app = App {
        scene: Box::from(ui::dev_setup::UIDevSetup::new()),
    };

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

struct App<'a> {
    scene: Box<dyn UIScene + 'a>,
}

impl App<'_> {
    fn run(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    ) -> Result<(), Box<dyn Error>> {
        //Install a Ctrl+C handler
        let should_quit = Arc::new(atomic::AtomicBool::new(false));
        {
            let should_quit = should_quit.clone();
            ctrlc::set_handler(move || should_quit.store(true, atomic::Ordering::SeqCst))?;
        }

        //Run the main UI loop
        while !should_quit.load(atomic::Ordering::SeqCst) && !self.scene.should_quit() {
            //Draw the current UI scene
            if TERMINAL_LOCK.load(atomic::Ordering::SeqCst) {
                terminal.draw(|frame| self.draw(frame))?;
            }

            //Poll for events
            if event::poll(Duration::from_millis(10))? {
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

    fn draw(&mut self, frame: &mut Frame) {
        //Calculate the layout
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(4), Constraint::Min(0)])
            .split(frame.size());

        //Draw the title
        frame.render_widget(
            Paragraph::new(vec![
                Line::from("WEPCrack".magenta().bold()),
                Line::from("WEP Key Cracking Demonstration Tool".blue()),
                Line::from("© Popax21, 2023".blue().italic()),
            ])
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::BOTTOM)),
            layout[0],
        );

        //Draw the scene
        self.scene.draw(frame, layout[1]);
    }
}

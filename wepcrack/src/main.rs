use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use rand::RngCore;
use ratatui::{prelude::CrosstermBackend, Terminal};
use std::{
    error::Error,
    sync::{
        atomic::{self, AtomicBool},
        Arc,
    },
    time::Duration,
};
use ui::UIScene;

use crate::{keycracker::KeystreamSample, wep::WepKey};

pub mod util;

pub mod keycracker;
pub mod rc4;
pub mod wep;

pub mod ui;

const KEYCRACK_SETTINGS: ui::keycracker::KeyCrackerSettings = ui::keycracker::KeyCrackerSettings {
    key_predictor_threshold: 0.5,

    num_test_samples: 2048,
    test_sample_period: 128,
    test_sample_threshold: 0.9,
};

static TERMINAL_LOCK: AtomicBool = AtomicBool::new(true);

fn main() -> Result<(), Box<dyn Error>> {
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
    let mut test_key = [0u8; WepKey::LEN_104];
    rand::thread_rng().fill_bytes(&mut test_key);
    let test_key: WepKey = WepKey::Wep104Key(test_key);

    let mut random_sample_provider = move || {
        //Generate a random sample from a random IV
        let mut sample = KeystreamSample::default();
        rand::thread_rng().fill_bytes(&mut sample.iv);
        test_key
            .create_rc4(&sample.iv)
            .gen_keystream(&mut sample.keystream);

        sample
    };

    let mut app = App {
        scene: Box::from(ui::keycracker::UIKeyCracker::new(
            KEYCRACK_SETTINGS,
            &mut random_sample_provider,
        )),
    };
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
                terminal.draw(|f| self.scene.draw(f))?;
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
}

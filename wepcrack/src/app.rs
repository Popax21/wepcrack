use crate::ieee80211::IEEE80211Monitor;
use crate::ui::UIScene;
use crate::TERMINAL_LOCK;
use crate::{nl80211::NL80211Connection, ui};

use anyhow::Context;
use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    prelude::{Alignment, Constraint, CrosstermBackend, Direction, Layout},
    style::Stylize,
    text::Line,
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};
use std::rc::{Rc, Weak};
use std::{
    cell::RefCell,
    error::Error,
    sync::{atomic, Arc},
    time::Duration,
};

struct AppState {
    state_ref: Weak<RefCell<AppState>>,

    new_scene: Option<Box<dyn UIScene>>,

    nl80211_con: Option<NL80211Connection>,
    ieee80211_mon: Option<IEEE80211Monitor>,
}

impl AppState {
    fn new() -> Rc<RefCell<AppState>> {
        let state = Rc::new(RefCell::new(AppState {
            state_ref: Weak::default(),
            new_scene: None,
            nl80211_con: None,
            ieee80211_mon: None,
        }));

        state.borrow_mut().state_ref = Rc::downgrade(&state);

        state
    }

    fn select_device(&mut self) {
        //Switch the scene to the device selection scene
        let state_ref = self.state_ref.clone();
        self.new_scene = Some(Box::new(ui::dev_select::UIDeviceSelect::new(
            self.nl80211_con.as_ref().unwrap(),
            Box::new(move |wiphy| {
                //Deref the state reference
                let Some(state) = state_ref.upgrade() else {
                    return;
                };
                let mut state = state.borrow_mut();

                //Create the IEEE 802.11 monitor
                state.ieee80211_mon = Some(
                    IEEE80211Monitor::enter_monitor_mode(state.nl80211_con.take().unwrap(), wiphy)
                        .expect("failed to create IEEE 802.11 monitor"),
                );
                println!(
                    "packet: {:?}",
                    state
                        .ieee80211_mon
                        .as_mut()
                        .unwrap()
                        .sniff_packet()
                        .expect("a")
                );

                //Switch the scene to the target selector
                state.select_target();
            }),
        )));
    }

    fn select_target(&mut self) {
        //Switch the scene to the device selection scene
        let state_ref = self.state_ref.clone();
        self.new_scene = Some(Box::new(ui::target_select::UITargetSelect::new()));
    }
}

pub struct App {
    scene: Box<dyn UIScene>,
    state: Rc<RefCell<AppState>>,
}

impl App {
    pub fn create() -> Result<App, Box<dyn Error>> {
        //Create a new nl80211 connection
        let nl80211_con =
            NL80211Connection::new().context("failed to create a nl80211 connection")?;

        //Allocate the app state
        let state_rc = AppState::new();

        //Set up the initial app state
        {
            let mut state = state_rc.borrow_mut();
            state.nl80211_con = Some(nl80211_con);
            state.select_device();
        }

        let scene = state_rc.borrow_mut().new_scene.take().unwrap();
        Ok(App {
            scene: scene,
            state: state_rc,
        })
    }

    fn switch_scenes(&mut self) {
        if let Some(new_scene) = self.state.borrow_mut().new_scene.take() {
            self.scene = new_scene;
        }
    }

    pub fn run(
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

                    self.switch_scenes();
                }
            }

            self.switch_scenes();
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

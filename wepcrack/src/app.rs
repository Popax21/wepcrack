use crate::arp_supplier::ARPSampleSupplier;
use crate::ieee80211::IEEE80211Monitor;
use crate::keycracker::KeystreamSample;
use crate::ui::keycracker::KeyCrackerSettings;
use crate::ui::UIScene;
use crate::wep::{WepIV, WepKey};
use crate::TERMINAL_LOCK;
use crate::{nl80211::NL80211Connection, ui};

use anyhow::Context;
use crossterm::event::{self, Event, KeyCode};
use hex::FromHex;
use rand::RngCore;
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

    nl80211_con: Rc<NL80211Connection>,
    ieee80211_mon: Option<Rc<IEEE80211Monitor>>,
}

impl AppState {
    fn new(nl80211_con: NL80211Connection) -> Rc<RefCell<AppState>> {
        Rc::new_cyclic(|state_ref| {
            RefCell::new(AppState {
                state_ref: state_ref.clone(),
                new_scene: None,
                nl80211_con: Rc::new(nl80211_con),
                ieee80211_mon: None,
            })
        })
    }

    fn select_device(&mut self) {
        //Switch the scene to the device selection scene
        let state_ref = self.state_ref.clone();
        self.new_scene = Some(Box::new(ui::dev_select::UIDeviceSelect::new(
            self.nl80211_con.as_ref(),
            move |wiphy| {
                //Deref the state reference
                let Some(state) = state_ref.upgrade() else {
                    return;
                };
                let mut state = state.borrow_mut();

                //Create the 802.11 monitor
                state.ieee80211_mon = Some(Rc::new(
                    IEEE80211Monitor::enter_monitor_mode(state.nl80211_con.clone(), wiphy)
                        .expect("failed to create 802.11 monitor"),
                ));

                //Switch the scene to the target selector
                state.select_target();
            },
        )));
    }

    fn select_target(&mut self) {
        //Switch the scene to the device selection scene
        let ieee80211_mon = self
            .ieee80211_mon
            .as_ref()
            .expect("no 802.11 monitor has been created")
            .clone();

        let state_ref = self.state_ref.clone();
        self.new_scene = Some(Box::new(ui::target_select::UITargetSelect::new(
            ieee80211_mon,
            move |ap_mac, dev_mac| {
                //Deref the state reference
                let Some(state) = state_ref.upgrade() else {
                    return;
                };
                let mut state = state.borrow_mut();

                //Switch the scene to attack preparation
                state.attack_preparation(ap_mac, dev_mac);
            },
        )));
    }

    fn attack_preparation(
        &mut self,
        ap_mac: ieee80211::MacAddress,
        dev_mac: ieee80211::MacAddress,
    ) {
        //Switch the scene to the attack preparation scene
        let ieee80211_mon = self
            .ieee80211_mon
            .as_ref()
            .expect("no 802.11 monitor has been created")
            .clone();

        let state_ref = self.state_ref.clone();
        self.new_scene = Some(Box::new(ui::attack_prep::UIAttackPrep::new(
            ieee80211_mon,
            ap_mac,
            dev_mac,
            move |prov| {
                //Deref the state reference
                let Some(state) = state_ref.upgrade() else {
                    return;
                };
                let mut state = state.borrow_mut();

                //Switch the scene to key cracking
                state.keycrack(prov);
            },
        )));
    }

    fn keycrack(&mut self, mut sample_prov: ARPSampleSupplier) {
        //Switch the scene to the key cracking scene
        const KEYCRACK_SETTINGS: KeyCrackerSettings = KeyCrackerSettings {
            key_predictor_normal_threshold: 0.075,
            key_predictor_strong_threshold: 0.025,
            num_test_samples: 1024,
            test_sample_period: 128,
            test_sample_threshold: 0.25,
        };

        self.new_scene = Some(Box::new(ui::keycracker::UIKeyCracker::new(
            KEYCRACK_SETTINGS,
            Box::new(move |should_exit| sample_prov.provide_sample(should_exit)),
        )));
    }

    fn keycrack_simulate(&mut self, key: WepKey) {
        //Switch the scene to the key cracking scene
        const KEYCRACK_SETTINGS: KeyCrackerSettings = KeyCrackerSettings {
            key_predictor_normal_threshold: 0.50,
            key_predictor_strong_threshold: 0.35,
            num_test_samples: 1024,
            test_sample_period: 128,
            test_sample_threshold: 1.,
        };

        self.new_scene = Some(Box::new(ui::keycracker::UIKeyCracker::new(
            KEYCRACK_SETTINGS,
            Box::new(move |_should_exit| {
                let mut iv = WepIV::default();
                rand::thread_rng().fill_bytes(&mut iv);

                let mut keystream = [0u8; 16];
                key.create_rc4(&iv).gen_keystream(&mut keystream);

                Some(KeystreamSample { iv, keystream })
            }),
        )));
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
        let state_rc = AppState::new(nl80211_con);

        if let Ok(simulate_key) = std::env::var("WEPCRACK_SIMULATE_KEY") {
            let simulate_key = match simulate_key.len() {
                10 => WepKey::Wep40Key(
                    <[u8; WepKey::LEN_40]>::from_hex(simulate_key)
                        .expect("failed to parse WEP key"),
                ),
                26 => WepKey::Wep104Key(
                    <[u8; WepKey::LEN_104]>::from_hex(simulate_key)
                        .expect("failed to parse WEP key"),
                ),
                _ => panic!("invalid WEP key length"),
            };
            state_rc.borrow_mut().keycrack_simulate(simulate_key);
        } else {
            state_rc.borrow_mut().select_device();
        }

        let scene = state_rc.borrow_mut().new_scene.take().unwrap();
        Ok(App {
            scene,
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
                Line::from(
                    "© Popax21, Marcel Gabriel Janzer 2023"
                        .blue()
                        .italic(),
                ),
            ])
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::BOTTOM)),
            layout[0],
        );

        //Draw the scene
        self.scene.draw(frame, layout[1]);
    }
}

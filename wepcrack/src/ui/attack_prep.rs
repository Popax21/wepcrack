use std::{
    rc::Rc,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    thread::JoinHandle,
};

use ieee80211::MacAddress;
use ratatui::{
    prelude::Rect,
    style::Stylize,
    text::{Line, Text},
    widgets::Paragraph,
    Frame,
};

use crate::{
    arp_supplier::ARPSampleSupplier,
    ieee80211::{IEEE80211Monitor, IEEE80211PacketSniffer},
};

use super::{draw_ui_widgets, ConfirmationWidget, UIScene};

enum PreparationStage {
    InitialPrompt(ConfirmationWidget<'static, ()>),
    SecondPrompt(ConfirmationWidget<'static, ()>),
    DeniedConfirmation,
    DidConfirm,
}

pub struct UIAttackPrep {
    prep_stage: PreparationStage,

    monitor: Rc<IEEE80211Monitor>,
    ap_mac: MacAddress,
    dev_mac: MacAddress,

    thread: Option<JoinHandle<ieee80211::Frame<'static>>>,
    prep_attempt: Arc<AtomicUsize>,

    callback: Option<Box<dyn FnOnce(ARPSampleSupplier)>>,
}

impl UIAttackPrep {
    pub fn new(
        monitor: Rc<IEEE80211Monitor>,
        ap_mac: MacAddress,
        dev_mac: MacAddress,
        callback: impl FnOnce(ARPSampleSupplier) + 'static,
    ) -> UIAttackPrep {
        UIAttackPrep {
            prep_stage: PreparationStage::InitialPrompt(ConfirmationWidget::new(Text::from(vec![
                Line::from(vec![
                    "Are you sure you want to launch an attack on Access Point ".into(),
                    ap_mac.to_hex_string().bold(),
                    " / target device ".into(),
                    dev_mac.to_hex_string().bold(),
                    "?".into(),
                ]),
                "ONLY CONTINUE IF YOU HAVE THE LEGAL RIGHTS TO DO SO!".into(),
            ]))),

            monitor,
            ap_mac,
            dev_mac,

            thread: None,
            prep_attempt: Arc::new(AtomicUsize::new(0)),

            callback: Some(Box::new(callback)),
        }
    }
}

impl UIScene for UIAttackPrep {
    fn should_quit(&self) -> bool {
        matches!(&self.prep_stage, PreparationStage::DeniedConfirmation)
            || self
                .thread
                .as_ref()
                .map_or(false, |thread| thread.is_finished())
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) {
        match &mut self.prep_stage {
            //Make the user confirm the attack first
            PreparationStage::InitialPrompt(confirm_widget)
            | PreparationStage::SecondPrompt(confirm_widget) => {
                draw_ui_widgets(&mut [confirm_widget], &(), frame, area);
            }
            PreparationStage::DeniedConfirmation => {}

            PreparationStage::DidConfirm => {
                //Check if the thread is done
                let attempt = self.prep_attempt.load(Ordering::SeqCst);
                if attempt == usize::MAX {
                    if let Some(cb) = self.callback.take() {
                        cb(ARPSampleSupplier::new(
                            self.monitor
                                .create_sniffer()
                                .expect("failed to create sniffer for ARP sample supplier"),
                            self.dev_mac,
                            self.thread.take().unwrap().join().unwrap(),
                        ))
                    }
                    return;
                }

                //Draw the attempt counter
                frame.render_widget(
                    Paragraph::new(vec![
                        "Attempting to obtain ARP request through deauth injection..."
                            .bold()
                            .into(),
                        format!("Attempt {attempt}").into(),
                    ]),
                    area,
                )
            }
        }
    }

    fn handle_event(&mut self, event: &crossterm::event::Event) {
        //Make the user confirm the attack first
        match &mut self.prep_stage {
            PreparationStage::InitialPrompt(confirm_widget) => {
                if let Some(confirm_res) = confirm_widget.handle_event(event) {
                    self.prep_stage = if confirm_res {
                        PreparationStage::SecondPrompt(ConfirmationWidget::new(
                            "Are you sure? This is the final confirmation".into(),
                        ))
                    } else {
                        PreparationStage::DeniedConfirmation
                    }
                }
            }
            PreparationStage::SecondPrompt(confirm_widget) => {
                if let Some(confirm_res) = confirm_widget.handle_event(event) {
                    if confirm_res {
                        //Launch the prep thread
                        let mut sniffer = self
                            .monitor
                            .create_sniffer()
                            .expect("failed to create packet sniffer for prep thread");
                        let ap_mac = self.ap_mac;
                        let dev_mac = self.dev_mac;
                        let attempt = self.prep_attempt.clone();

                        self.thread = Some(std::thread::spawn(move || {
                            prep_thread_fnc(ap_mac, dev_mac, &mut sniffer, attempt.as_ref())
                        }));

                        self.prep_stage = PreparationStage::DidConfirm;
                    } else {
                        self.prep_stage = PreparationStage::DeniedConfirmation;
                    }
                }
            }
            _ => {}
        }
    }
}

fn prep_thread_fnc(
    ap_mac: MacAddress,
    dev_mac: MacAddress,
    sniffer: &mut IEEE80211PacketSniffer,
    attempt: &AtomicUsize,
) -> ieee80211::Frame<'static> {
    loop {
        attempt.fetch_add(1, Ordering::SeqCst);

        if let Some(arp_req) =
            ARPSampleSupplier::try_capture_arp_request(&ap_mac, &dev_mac, sniffer)
                .expect("error while trying to capture ARP request")
        {
            attempt.store(usize::MAX, Ordering::SeqCst);
            return arp_req;
        }
    }
}

use std::{sync::Arc, thread::JoinHandle};

use crossterm::event::Event;
use ratatui::{
    prelude::{Alignment, Constraint, Direction, Layout, Rect},
    style::Stylize,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::{
    keycracker::{KeyBytePrediction, KeyCrackerSettings, KeystreamSampleProvider, WepKeyCracker},
    util::RecessiveMutex,
    wep::WepKey,
};

use super::UIScene;

struct KeyCrackerData<'a> {
    exit: bool,
    cracker: WepKeyCracker,
    sample_provider: &'a KeystreamSampleProvider,
}
pub struct UIKeyCrack<'a> {
    cracker_data: Arc<RecessiveMutex<KeyCrackerData<'a>>>,
    cracker_thread: Option<JoinHandle<()>>,
}

impl UIKeyCrack<'_> {
    pub fn new<'a>(
        keycrack_settings: &KeyCrackerSettings,
        sample_provider: &'a KeystreamSampleProvider,
    ) -> UIKeyCrack<'a> {
        //Initialize the key cracker data
        let cracker_data = KeyCrackerData {
            exit: false,
            cracker: WepKeyCracker::new(keycrack_settings),
            sample_provider,
        };
        let cracker_data = Arc::new(RecessiveMutex::new(cracker_data));

        //Launch the key cracker thread
        let cracker_thread = {
            let cracker_data = unsafe {
                //We know the thread is joined in the drop method, so the thread
                //will drop the Arc before 'a goes out of scope (since
                //UIKeyCrack can not live longer than 'a)
                std::mem::transmute::<_, Arc<RecessiveMutex<KeyCrackerData<'static>>>>(
                    cracker_data.clone(),
                )
            };

            std::thread::spawn(move || loop {
                //Lock the cracker data
                let mut cracker_data = cracker_data.lock_recessive().unwrap();

                //Exit if we should
                if cracker_data.exit {
                    return;
                }

                //Collect a sample and process it
                let sample = (cracker_data.sample_provider)();
                cracker_data.cracker.accept_sample(&sample);
            })
        };

        UIKeyCrack {
            cracker_data,
            cracker_thread: Some(cracker_thread),
        }
    }

    fn draw_sample_stats(&self, cracker_data: &KeyCrackerData, frame: &mut Frame, area: Rect) {
        let layout = Layout::default()
            .constraints([Constraint::Min(0)])
            .margin(1)
            .split(area);

        //Draw the border block
        frame.render_widget(
            Block::default()
                .borders(Borders::all())
                .title("Sample Stats"),
            area,
        );

        //Draw the stats text
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                "#samples: ".bold(),
                format!("{:8}", cracker_data.cracker.num_samples()).into(),
            ])),
            layout[0],
        );
    }

    fn draw_sigmas(&self, cracker_data: &KeyCrackerData, frame: &mut Frame, area: Rect) {
        let layout = Layout::default()
            .constraints([Constraint::Min(0)])
            .margin(1)
            .split(area);

        //Draw the border block
        frame.render_widget(
            Block::default().borders(Borders::all()).title("Sigma Sums"),
            area,
        );

        //Draw the list
        let mut sigma_list = Vec::<ListItem>::new();

        for i in 0..WepKey::LEN_104 {
            //Get key byte info
            let info = cracker_data.cracker.calc_key_byte_info(i);

            //Construct the info line
            let mut info_line = Vec::<Span<'_>>::new();

            info_line.extend_from_slice(&[
                "σ".cyan().bold(),
                "[".dark_gray(),
                format!("{i:2}").into(),
                "]".dark_gray(),
                ":".into(),
            ]);

            // - probabilities
            info_line.extend_from_slice(&[
                " candidate=".dark_gray(),
                format!("{:02x}", info.candidate_sigma).into(),
                " p_candidate=".dark_gray(),
                format!("{:.8}", info.p_candidate).into(),
                " p_correct=".dark_gray(),
                format!("{:.8}", info.p_correct).into(),
                " p_equal=".dark_gray(),
            ]);

            // - errors
            info_line.extend_from_slice(&[
                format!("{:.8}", info.p_equal).into(),
                " err_normal=".dark_gray(),
                format!("{:1.9}", info.err_normal).into(),
                " err_strong=".dark_gray(),
                format!("{:1.9}", info.err_strong).into(),
            ]);

            // - prediction
            info_line.push(" pred: ".dark_gray());
            match info.get_prediction() {
                KeyBytePrediction::Normal { sigma: _ } => info_line.push("normal".magenta().bold()),
                KeyBytePrediction::Strong => info_line.push("strong".cyan().bold()),
            }
            info_line.push(format!(" {:3.3}%", info.get_prediction_score() * 100.).into());

            //Create the list item
            sigma_list.push(ListItem::new(Line::from(info_line)));
        }

        frame.render_widget(List::new(sigma_list), layout[0]);
    }
}

impl Drop for UIKeyCrack<'_> {
    fn drop(&mut self) {
        //Stop the key cracker thread
        if let Ok(mut cracker_data) = self.cracker_data.lock_dominant() {
            cracker_data.exit = true;
        }

        if let Some(handle) = self.cracker_thread.take() {
            if let Err(err) = handle.join() {
                std::panic::resume_unwind(err);
            }
        }
    }
}

impl UIScene for UIKeyCrack<'_> {
    fn should_quit(&self) -> bool {
        if let Some(thread) = self.cracker_thread.as_ref() {
            thread.is_finished()
        } else {
            true
        }
    }

    fn draw_ui(&mut self, frame: &mut Frame) {
        //Calculate the layout
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4),
                Constraint::Length(3),
                Constraint::Length((1 + WepKey::LEN_104 + 1) as u16),
                Constraint::Min(0),
            ])
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

        //Lock the key cracker data
        let Ok(cracker_data) = self.cracker_data.lock_dominant() else {
            return;
        };

        //Draw sample statistics
        self.draw_sample_stats(&cracker_data, frame, layout[1]);

        //Draw sigma sums
        self.draw_sigmas(&cracker_data, frame, layout[2]);
    }

    fn handle_event(&mut self, _event: &Event) {}
}

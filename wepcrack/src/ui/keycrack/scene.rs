use std::iter;

use crossterm::event::Event;
use ratatui::{
    prelude::{Alignment, Constraint, Direction, Layout},
    style::Stylize,
    text::Line,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::{
    keycracker::{KeyCrackerSettings, KeystreamSampleProvider},
    ui::UIScene,
};

use super::{
    KeyCrackPhase, KeyCrackWidget, KeyCrackerThread, KeyCrackerThreadData, OverviewWidget,
    SigmaInfoWidget,
};

struct KeyCrackWidgets {
    overview_widget: OverviewWidget,
    sigma_info_widget: SigmaInfoWidget,
}

impl KeyCrackWidgets {
    fn get_ui_widgets(&mut self) -> Vec<&mut dyn KeyCrackWidget> {
        vec![&mut self.overview_widget, &mut self.sigma_info_widget]
    }
}

pub struct UIKeyCrack<'a> {
    cracker_thread: KeyCrackerThread<'a>,
    widgets: KeyCrackWidgets,
}

impl<'d> UIKeyCrack<'d> {
    pub fn new<'a>(
        cracker_settings: &KeyCrackerSettings,
        sample_provider: &'a KeystreamSampleProvider,
    ) -> UIKeyCrack<'a> {
        UIKeyCrack {
            cracker_thread: KeyCrackerThread::launch(cracker_settings, sample_provider),

            widgets: KeyCrackWidgets {
                overview_widget: OverviewWidget::new(),
                sigma_info_widget: SigmaInfoWidget::new(),
            },
        }
    }

    fn advance_phase_if_done(&self, cracker_data: &mut KeyCrackerThreadData) {
        match cracker_data.phase() {
            KeyCrackPhase::SampleCollection => {
                //Check if the cracker is ready
                if cracker_data.cracker.is_ready() {
                    cracker_data.change_phase(KeyCrackPhase::KeyTesting);
                }
            }
            KeyCrackPhase::KeyTesting => {}
            KeyCrackPhase::Done => {}
        }
    }
}

impl UIScene for UIKeyCrack<'_> {
    fn should_quit(&self) -> bool {
        self.cracker_thread.did_crash()
    }

    fn draw(&mut self, frame: &mut Frame) {
        //Lock the key cracker thread data
        let Ok(mut cracker_data) = self.cracker_thread.lock_data() else {
            return;
        };

        //Advance the cracker phase
        self.advance_phase_if_done(&mut cracker_data);

        //Get the UI widget list
        let mut widgets = self.widgets.get_ui_widgets();

        //Calculate the layout
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                iter::once(Constraint::Length(3))
                    .chain(widgets.iter().map(|w| w.size()))
                    .chain(iter::once(Constraint::Min(0)))
                    .collect::<Vec<_>>(),
            )
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
        let layout = &layout[1..];

        //Draw widgets
        for (i, widget) in widgets.iter_mut().enumerate() {
            widget.draw(&cracker_data, frame, layout[i]);
        }
    }

    fn handle_event(&mut self, _event: &Event) {}
}

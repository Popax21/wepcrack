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
    ui::{UIScene, UIWidget},
};

use super::{KeyCrackerThread, KeyCrackerThreadData, SampleStatsWidget, SigmaInfoWidget};

pub struct UIKeyCrack<'a> {
    cracker_thread: KeyCrackerThread<'a>,
}

impl<'d> UIKeyCrack<'d> {
    pub fn new<'a>(
        cracker_settings: &KeyCrackerSettings,
        sample_provider: &'a KeystreamSampleProvider,
    ) -> UIKeyCrack<'a> {
        UIKeyCrack {
            cracker_thread: KeyCrackerThread::launch(cracker_settings, sample_provider),
        }
    }

    fn create_ui_widgets<'a>(
        &self,
        cracker_data: &'a KeyCrackerThreadData<'d>,
    ) -> Vec<Box<dyn UIWidget + 'a>> {
        vec![
            Box::new(SampleStatsWidget::new(&cracker_data.cracker)),
            Box::new(SigmaInfoWidget::new(&cracker_data.cracker)),
        ]
    }
}

impl UIScene for UIKeyCrack<'_> {
    fn should_quit(&self) -> bool {
        self.cracker_thread.did_crash()
    }

    fn draw(&self, frame: &mut Frame) {
        //Lock the key cracker thread data
        let Ok(cracker_data) = self.cracker_thread.lock_data() else {
            return;
        };

        //Create widget list
        let widgets = self.create_ui_widgets(&cracker_data);

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
        for (i, widget) in widgets.iter().enumerate() {
            widget.draw(frame, layout[i]);
        }
    }

    fn handle_event(&mut self, _event: &Event) {}
}

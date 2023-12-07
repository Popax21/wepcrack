use std::iter;

use crossterm::event::Event;
use ratatui::{
    prelude::{Alignment, Constraint, Direction, Layout, Rect},
    style::Stylize,
    text::Line,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::ui::UIScene;

use super::{
    CandidateKeyTestingWidget, KeyCracker, KeyCrackerPhase, KeyCrackerSampleProvider,
    KeyCrackerSettings, KeyCrackerThread, OverviewWidget, SigmaInfoWidget,
};

pub(super) trait KeyCrackerWidget {
    fn size(&self) -> Constraint;
    fn draw(&mut self, cracker: &KeyCracker, frame: &mut Frame, area: Rect);
}

struct KeyCrackerWidgets {
    overview_widget: OverviewWidget,
    sigma_info_widget: SigmaInfoWidget,
    candidate_testing_widget: CandidateKeyTestingWidget,
}

impl KeyCrackerWidgets {
    fn get_ui_widgets(&mut self, cracker: &KeyCracker) -> Vec<&mut dyn KeyCrackerWidget> {
        match cracker.phase() {
            KeyCrackerPhase::SampleCollection => {
                vec![&mut self.overview_widget, &mut self.sigma_info_widget]
            }
            _ => vec![
                &mut self.overview_widget,
                &mut self.sigma_info_widget,
                &mut self.candidate_testing_widget,
            ],
        }
    }
}

pub struct UIKeyCracker<'a> {
    cracker_thread: KeyCrackerThread<'a>,
    widgets: KeyCrackerWidgets,
}

impl UIKeyCracker<'_> {
    pub fn new(
        cracker_settings: KeyCrackerSettings,
        sample_provider: &mut KeyCrackerSampleProvider,
    ) -> UIKeyCracker {
        UIKeyCracker {
            cracker_thread: KeyCrackerThread::launch(cracker_settings, sample_provider),

            widgets: KeyCrackerWidgets {
                overview_widget: OverviewWidget::new(),
                sigma_info_widget: SigmaInfoWidget::new(),
                candidate_testing_widget: CandidateKeyTestingWidget::new(),
            },
        }
    }
}

impl UIScene for UIKeyCracker<'_> {
    fn should_quit(&self) -> bool {
        self.cracker_thread.did_crash()
    }

    fn draw(&mut self, frame: &mut Frame) {
        //Lock the key cracker thread data
        let Ok(cracker) = self.cracker_thread.lock_state() else {
            return;
        };

        //Get the UI widget list
        let mut widgets = self.widgets.get_ui_widgets(&cracker);

        //Calculate the layout
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                iter::once(Constraint::Length(4))
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
            widget.draw(&cracker, frame, layout[i]);
        }
    }

    fn handle_event(&mut self, _event: &Event) {}
}

use crossterm::event::Event;
use ratatui::{prelude::Rect, Frame};

use crate::ui::{draw_ui_widgets, UIScene};

use super::{
    CandidateKeyTestingWidget, KeyCrackerPhase, KeyCrackerSampleProvider, KeyCrackerSettings,
    KeyCrackerThread, OverviewWidget, SigmaInfoWidget,
};

pub struct UIKeyCracker {
    cracker_thread: KeyCrackerThread,

    overview_widget: OverviewWidget,
    sigma_info_widget: SigmaInfoWidget,
    candidate_testing_widget: CandidateKeyTestingWidget,
}

impl UIKeyCracker {
    pub fn new(
        cracker_settings: KeyCrackerSettings,
        sample_provider: Box<KeyCrackerSampleProvider>,
    ) -> UIKeyCracker {
        UIKeyCracker {
            cracker_thread: KeyCrackerThread::launch(cracker_settings, sample_provider),

            overview_widget: OverviewWidget::new(),
            sigma_info_widget: SigmaInfoWidget::new(),
            candidate_testing_widget: CandidateKeyTestingWidget::new(),
        }
    }
}

impl UIScene for UIKeyCracker {
    fn should_quit(&self) -> bool {
        self.cracker_thread.did_crash()
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) {
        //Lock the key cracker thread data
        let Ok(cracker) = self.cracker_thread.lock_state() else {
            return;
        };

        //Draw widgets
        if cracker.phase() < KeyCrackerPhase::CandidateKeyTesting {
            draw_ui_widgets(
                &mut [&mut self.overview_widget, &mut self.sigma_info_widget],
                &cracker,
                frame,
                area,
            );
        } else {
            draw_ui_widgets(
                &mut [
                    &mut self.overview_widget,
                    &mut self.sigma_info_widget,
                    &mut self.candidate_testing_widget,
                ],
                &cracker,
                frame,
                area,
            );
        }
    }

    fn handle_event(&mut self, _event: &Event) {}
}

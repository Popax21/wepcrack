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
    KeyCracker, KeyCrackerSampleProvider, KeyCrackerSettings, KeyCrackerThread, OverviewWidget,
    SigmaInfoWidget,
};

pub(crate) trait KeyCrackWidget {
    fn size(&self) -> Constraint;
    fn draw(&mut self, cracker: &KeyCracker, frame: &mut Frame, area: Rect);
}

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
        cracker_settings: KeyCrackerSettings,
        sample_provider: &'a mut KeyCrackerSampleProvider,
    ) -> UIKeyCrack<'a> {
        UIKeyCrack {
            cracker_thread: KeyCrackerThread::launch(cracker_settings, sample_provider),

            widgets: KeyCrackWidgets {
                overview_widget: OverviewWidget::new(),
                sigma_info_widget: SigmaInfoWidget::new(),
            },
        }
    }
}

impl UIScene for UIKeyCrack<'_> {
    fn should_quit(&self) -> bool {
        self.cracker_thread.did_crash()
    }

    fn draw(&mut self, frame: &mut Frame) {
        //Lock the key cracker thread data
        let Ok(cracker) = self.cracker_thread.lock_state() else {
            return;
        };

        //Get the UI widget list
        let mut widgets = self.widgets.get_ui_widgets();

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

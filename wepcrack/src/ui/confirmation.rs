use std::marker::PhantomData;

use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{
    prelude::{Constraint, Layout, Rect},
    style::Stylize,
    text::Line,
    widgets::Paragraph,
    Frame,
};

use crate::ui::draw_ui_widget_border;

use super::UIWidget;

pub struct ConfirmationWidget<'a, S> {
    message: Line<'a>,
    selected_opt: bool,

    _s: PhantomData<S>,
}

impl<'a, S> ConfirmationWidget<'a, S> {
    pub fn new(message: Line<'a>) -> ConfirmationWidget<'a, S> {
        ConfirmationWidget {
            message,
            selected_opt: false,
            _s: PhantomData,
        }
    }

    pub fn handle_event(&mut self, event: &Event) -> Option<bool> {
        let Event::Key(key) = event else {
            return None;
        };
        if key.kind == KeyEventKind::Release {
            return None;
        }

        match key.code {
            KeyCode::Up | KeyCode::Down => {
                self.selected_opt ^= true;
                None
            }
            KeyCode::Enter => Some(self.selected_opt),
            _ => None,
        }
    }
}

impl<S> UIWidget<'_> for ConfirmationWidget<'_, S> {
    type SharedState = S;

    fn size(&self, _: &S) -> u16 {
        5
    }

    fn draw(&mut self, _: &S, frame: &mut Frame, area: Rect) {
        draw_ui_widget_border("Confirmation Request", frame, area);

        //Calculate the layout
        let [msg_area, yes_area, no_area] = *Layout::new()
            .margin(1)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(area)
        else {
            unreachable!();
        };

        //Draw the message
        frame.render_widget(Paragraph::new(self.message.clone()), msg_area);

        //Draw the confirmation options
        frame.render_widget(
            Paragraph::new(Line::from(if self.selected_opt {
                vec!["> ".into(), "yes".green().bold()]
            } else {
                vec!["  ".into(), "yes".green()]
            })),
            yes_area,
        );
        frame.render_widget(
            Paragraph::new(Line::from(if !self.selected_opt {
                vec!["> ".into(), "no".red().bold()]
            } else {
                vec!["  ".into(), "no".red()]
            })),
            no_area,
        );
    }
}

use crossterm::event::Event;
use ratatui::{widgets::Paragraph, Frame};

use super::UIScene;

pub struct UIKeycrack;

impl UIScene for UIKeycrack {
    fn draw_ui(&self, frame: &mut Frame) {
        frame.render_widget(Paragraph::new("Hello World!"), frame.size());
    }

    fn handle_event(&mut self, _event: &Event) {}
}

use crossterm::event::Event;
use ratatui::{prelude::Rect, Frame};

use crate::ui::UIScene;

pub struct UITargetSelect;
impl UITargetSelect {
    pub fn new() -> UITargetSelect {
        UITargetSelect
    }
}

impl UIScene for UITargetSelect {
    fn should_quit(&self) -> bool {
        false
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) {}

    fn handle_event(&mut self, event: &Event) {}
}

use crossterm::event::Event;
use ratatui::{
    layout::{Constraint, Rect},
    Frame,
};

pub trait UIApp {
    fn set_scene(&mut self, scene: impl UIScene);
}

pub trait UIScene {
    fn should_quit(&self) -> bool;

    fn draw(&self, frame: &mut Frame);
    fn handle_event(&mut self, event: &Event);
}

pub trait UIWidget {
    fn size(&self) -> Constraint;
    fn draw(&self, frame: &mut Frame, area: Rect);
}

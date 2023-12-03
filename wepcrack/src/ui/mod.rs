use crossterm::event::Event;
use ratatui::Frame;

pub trait UIApp {
    fn set_scene(&mut self, scene: impl UIScene);
}

pub trait UIScene {
    fn draw_ui(&self, frame: &mut Frame);
    fn handle_event(&mut self, event: &Event);
}

pub mod keycrack;

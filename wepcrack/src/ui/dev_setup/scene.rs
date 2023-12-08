use crossterm::event::Event;
use ratatui::{prelude::Rect, Frame};

use crate::ui::{draw_ui_widgets, UIScene};

use super::{DevListWidget, DevManager};

pub struct UIDevSetup {
    dev_manager: DevManager,

    dev_list_widget: DevListWidget,
}

impl UIDevSetup {
    pub fn new() -> UIDevSetup {
        UIDevSetup {
            dev_manager: DevManager::new().expect("failed to set up nl82011 device manager"),

            dev_list_widget: DevListWidget::new(),
        }
    }
}

impl UIScene for UIDevSetup {
    fn should_quit(&self) -> bool {
        false
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) {
        draw_ui_widgets(
            &mut [&mut self.dev_list_widget],
            &self.dev_manager,
            frame,
            area,
        )
    }

    fn handle_event(&mut self, event: &Event) {
        self.dev_list_widget.handle_event(&self.dev_manager, event);
    }
}

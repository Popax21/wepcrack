use crossterm::event::Event;
use ratatui::{prelude::Rect, Frame};

use crate::ui::{draw_ui_widgets, UIScene};

use super::{DevListWidget, DevManager, LogWidget};

pub struct UIDevSetup {
    dev_manager: DevManager,

    dev_list_widget: DevListWidget,
    log_widget: LogWidget,
}

impl UIDevSetup {
    const MAX_LOG_LINES: usize = 8;

    #[allow(clippy::new_without_default)]
    pub fn new() -> UIDevSetup {
        //Create the device manager
        let dev_manager = DevManager::new(UIDevSetup::MAX_LOG_LINES)
            .expect("failed to create the device manager");

        UIDevSetup {
            dev_list_widget: DevListWidget::new(&dev_manager),
            log_widget: LogWidget,

            dev_manager,
        }
    }
}

impl UIScene for UIDevSetup {
    fn should_quit(&self) -> bool {
        false
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) {
        if self.dev_manager.log_buffer().len() > 0 {
            draw_ui_widgets(
                &mut [&mut self.dev_list_widget, &mut self.log_widget],
                &self.dev_manager,
                frame,
                area,
            );
        } else {
            draw_ui_widgets(
                &mut [&mut self.dev_list_widget],
                &self.dev_manager,
                frame,
                area,
            );
        }
    }

    fn handle_event(&mut self, event: &Event) {
        self.dev_list_widget.handle_event(&self.dev_manager, event);
    }
}

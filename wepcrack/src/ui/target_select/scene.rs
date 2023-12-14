use std::rc::Rc;

use crossterm::event::Event;
use ratatui::{prelude::Rect, Frame};

use crate::{
    ieee80211::IEEE80211Monitor,
    ui::{draw_ui_widgets, UIScene},
};

use super::{TargetMonitor, UIChannelSelect};

pub struct UITargetSelect {
    monitor: TargetMonitor,

    channel_sel_widget: UIChannelSelect,
}

impl UITargetSelect {
    pub fn new(ieee_monitor: Rc<IEEE80211Monitor>) -> UITargetSelect {
        //Set up the target monitor
        let monitor = TargetMonitor::new(ieee_monitor);

        UITargetSelect {
            monitor,
            channel_sel_widget: UIChannelSelect::new(),
        }
    }
}

impl UIScene for UITargetSelect {
    fn should_quit(&self) -> bool {
        false
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) {
        draw_ui_widgets(
            &mut [&mut self.channel_sel_widget],
            &self.monitor,
            frame,
            area,
        )
    }

    fn handle_event(&mut self, event: &Event) {
        self.channel_sel_widget.handle_event(&self.monitor, event);
    }
}

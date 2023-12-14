use std::rc::Rc;

use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{prelude::Rect, style::Stylize, text::Line, Frame};

use crate::{
    ieee80211::IEEE80211Monitor,
    ui::{draw_ui_widgets, ConfirmationWidget, UIScene},
};

use super::{TargetMonitor, UIChannelSelect};

pub enum TargetSelectState {
    ChannelSelect {
        channel_sel_widget: UIChannelSelect,
        confirmation_widget: Option<ConfirmationWidget<'static, TargetMonitor>>,
    },
}

impl TargetSelectState {
    pub fn channel_select(monitor: &TargetMonitor) -> TargetSelectState {
        Self::ChannelSelect {
            channel_sel_widget: UIChannelSelect::new(monitor),
            confirmation_widget: None,
        }
    }
}

pub struct UITargetSelect {
    monitor: TargetMonitor,
    state: TargetSelectState,
}

impl UITargetSelect {
    pub fn new(ieee_monitor: Rc<IEEE80211Monitor>) -> UITargetSelect {
        //Set up the target monitor
        let monitor = TargetMonitor::new(ieee_monitor);

        //Set up the initial state
        let state = TargetSelectState::channel_select(&monitor);

        UITargetSelect { monitor, state }
    }
}

impl UIScene for UITargetSelect {
    fn should_quit(&self) -> bool {
        false
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) {
        //Draw different widgets depending on the current state
        match &mut self.state {
            TargetSelectState::ChannelSelect {
                channel_sel_widget,
                confirmation_widget,
            } => {
                //Draw channel select widgets
                if let Some(confirmation_widget) = confirmation_widget {
                    draw_ui_widgets(
                        &mut [channel_sel_widget, confirmation_widget],
                        &self.monitor,
                        frame,
                        area,
                    );
                } else {
                    draw_ui_widgets(&mut [channel_sel_widget], &self.monitor, frame, area);
                }
            }
        }
    }

    fn handle_event(&mut self, event: &Event) {
        //Run different event handlers depending on the current state
        match &mut self.state {
            TargetSelectState::ChannelSelect {
                channel_sel_widget,
                confirmation_widget: confirmation_widget_opt,
            } => {
                //Handle channel select inputs
                if let Some(confirmation_widget) = confirmation_widget_opt {
                    if let Some(confirm_res) = confirmation_widget.handle_event(event) {
                        if confirm_res {
                            //Switch to the new channel
                            self.monitor
                                .set_channel(*channel_sel_widget.selected_channel(&self.monitor))
                                .expect("failed to set active channel");

                            //Move onto the actual target selection
                            self.state = TargetSelectState::channel_select(&self.monitor);
                        } else {
                            *confirmation_widget_opt = None;
                        }
                    }
                } else {
                    //Ask for confirmation upon pressing enter
                    if let Event::Key(key) = event {
                        if key.kind == KeyEventKind::Press && key.code == KeyCode::Enter {
                            let channel = channel_sel_widget.selected_channel(&self.monitor);

                            *confirmation_widget_opt =
                                Some(ConfirmationWidget::new(Line::from(vec![
                                    "Do you want to switch to WiFi channel ".into(),
                                    channel.to_string().bold(),
                                    "?".into(),
                                ])));
                            return;
                        }
                    }

                    channel_sel_widget.handle_event(&self.monitor, event);
                }
            }
        }
    }
}

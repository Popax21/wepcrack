use std::rc::Rc;

use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{prelude::Rect, text::Line, Frame};

use crate::{
    ieee80211::IEEE80211Monitor,
    ui::{draw_ui_widgets, ConfirmationWidget, UIScene},
};

use super::{TargetMonitor, UIAccessPointList, UIChannelSelect};

pub enum TargetSelectState {
    ChannelSelect {
        channel_list_widget: UIChannelSelect,
        confirmation_widget: Option<ConfirmationWidget<'static, TargetMonitor>>,
    },
    APSelect {
        ap_list_widget: UIAccessPointList,
        confirmation_widget: Option<ConfirmationWidget<'static, TargetMonitor>>,
    },
}

impl TargetSelectState {
    pub fn channel_select(monitor: &TargetMonitor) -> TargetSelectState {
        Self::ChannelSelect {
            channel_list_widget: UIChannelSelect::new(monitor),
            confirmation_widget: None,
        }
    }

    pub fn ap_select(monitor: &TargetMonitor) -> TargetSelectState {
        Self::APSelect {
            ap_list_widget: UIAccessPointList::new(monitor),
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
        self.monitor.did_crash()
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) {
        //Draw different widgets depending on the current state
        match &mut self.state {
            TargetSelectState::ChannelSelect {
                channel_list_widget: channel_sel_widget,
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

            TargetSelectState::APSelect {
                ap_list_widget: ap_sel_widget,
                confirmation_widget,
            } => {
                //Draw access point select widgets
                if let Some(confirmation_widget) = confirmation_widget {
                    draw_ui_widgets(
                        &mut [ap_sel_widget, confirmation_widget],
                        &self.monitor,
                        frame,
                        area,
                    );
                } else {
                    draw_ui_widgets(&mut [ap_sel_widget], &self.monitor, frame, area);
                }
            }
        }
    }

    fn handle_event(&mut self, event: &Event) {
        //Run different event handlers depending on the current state
        match &mut self.state {
            TargetSelectState::ChannelSelect {
                channel_list_widget: channel_sel_widget,
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

                            //Start sniffing APs
                            self.monitor.sniff_aps();

                            //Move onto selecting the access point
                            self.state = TargetSelectState::ap_select(&self.monitor);
                        } else {
                            *confirmation_widget_opt = None;
                        }
                    }
                } else {
                    //Ask for confirmation upon pressing enter
                    if let Event::Key(key) = event {
                        if key.kind == KeyEventKind::Press && key.code == KeyCode::Enter {
                            *confirmation_widget_opt = Some(ConfirmationWidget::new(Line::from(
                                "Do you want to switch to the selected WiFi channel?",
                            )));
                            return;
                        }
                    }

                    channel_sel_widget.handle_event(&self.monitor, event);
                }
            }

            TargetSelectState::APSelect {
                ap_list_widget: ap_sel_widget,
                confirmation_widget: confirmation_widget_opt,
            } => {
                //Handle access point select inputs
                if let Some(confirmation_widget) = confirmation_widget_opt {
                    if let Some(confirm_res) = confirmation_widget.handle_event(event) {
                        if confirm_res {
                            todo!();
                        } else {
                            *confirmation_widget_opt = None;
                        }
                    }
                } else {
                    //Ask for confirmation upon pressing enter
                    ap_sel_widget.handle_event(&self.monitor, event);
                }
            }
        }
    }
}

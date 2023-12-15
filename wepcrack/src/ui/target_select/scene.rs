use std::rc::Rc;

use crossterm::event::{Event, KeyCode, KeyEventKind};
use ieee80211::MacAddress;
use ratatui::{prelude::Rect, style::Stylize, text::Line, Frame};

use crate::{
    ieee80211::IEEE80211Monitor,
    ui::{draw_ui_widgets, ConfirmationWidget, UIScene},
};

use super::{TargetMonitor, UIAccessPointList, UIChannelSelect, UITargetDeviceList};

pub enum TargetSelectState {
    ChannelSelect {
        channel_list_widget: UIChannelSelect,
        confirmation_widget: Option<ConfirmationWidget<'static, TargetMonitor>>,
    },
    APSelect {
        ap_list_widget: UIAccessPointList,
        confirmation_widget: Option<ConfirmationWidget<'static, TargetMonitor>>,
    },
    DevSelect {
        target_ap_mac: MacAddress,

        dev_list_widget: UITargetDeviceList,
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

    pub fn dev_select(target_ap_mac: MacAddress, monitor: &TargetMonitor) -> TargetSelectState {
        Self::DevSelect {
            target_ap_mac,

            dev_list_widget: UITargetDeviceList::new(monitor),
            confirmation_widget: None,
        }
    }
}

pub struct UITargetSelect {
    monitor: TargetMonitor,
    state: TargetSelectState,
    callback: Option<Box<dyn FnOnce(MacAddress, MacAddress)>>,
}

impl UITargetSelect {
    pub fn new(
        ieee_monitor: Rc<IEEE80211Monitor>,
        callback: impl FnOnce(MacAddress, MacAddress) + 'static,
    ) -> UITargetSelect {
        //Set up the target monitor
        let monitor = TargetMonitor::new(ieee_monitor);

        //Set up the initial state
        let state = TargetSelectState::channel_select(&monitor);

        UITargetSelect {
            monitor,
            state,
            callback: Some(Box::new(callback)),
        }
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
                channel_list_widget,
                confirmation_widget,
            } => {
                //Draw channel select widgets
                if let Some(confirmation_widget) = confirmation_widget {
                    draw_ui_widgets(
                        &mut [channel_list_widget, confirmation_widget],
                        &self.monitor,
                        frame,
                        area,
                    );
                } else {
                    draw_ui_widgets(&mut [channel_list_widget], &self.monitor, frame, area);
                }
            }

            TargetSelectState::APSelect {
                ap_list_widget,
                confirmation_widget,
            } => {
                //Draw access point select widgets
                if let Some(confirmation_widget) = confirmation_widget {
                    draw_ui_widgets(
                        &mut [ap_list_widget, confirmation_widget],
                        &self.monitor,
                        frame,
                        area,
                    );
                } else {
                    draw_ui_widgets(&mut [ap_list_widget], &self.monitor, frame, area);
                }
            }

            TargetSelectState::DevSelect {
                target_ap_mac: _,
                dev_list_widget,
                confirmation_widget,
            } => {
                //Draw target device select widgets
                if let Some(confirmation_widget) = confirmation_widget {
                    draw_ui_widgets(
                        &mut [dev_list_widget, confirmation_widget],
                        &self.monitor,
                        frame,
                        area,
                    );
                } else {
                    draw_ui_widgets(&mut [dev_list_widget], &self.monitor, frame, area);
                }
            }
        }
    }

    fn handle_event(&mut self, event: &Event) {
        //Run different event handlers depending on the current state
        match &mut self.state {
            TargetSelectState::ChannelSelect {
                channel_list_widget,
                confirmation_widget: confirmation_widget_opt,
            } => {
                //Handle channel select inputs
                if let Some(confirmation_widget) = confirmation_widget_opt {
                    if let Some(confirm_res) = confirmation_widget.handle_event(event) {
                        if confirm_res {
                            //Switch to the new channel
                            self.monitor
                                .set_channel(*channel_list_widget.selected_channel(&self.monitor))
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
                            *confirmation_widget_opt = Some(ConfirmationWidget::new(
                                "Do you want to switch to the selected WiFi channel?".into(),
                            ));
                            return;
                        }
                    }

                    channel_list_widget.handle_event(&self.monitor, event);
                }
            }

            TargetSelectState::APSelect {
                ap_list_widget,
                confirmation_widget: confirmation_widget_opt,
            } => {
                //Handle access point select inputs
                if let Some(confirmation_widget) = confirmation_widget_opt {
                    if let Some(confirm_res) = confirmation_widget.handle_event(event) {
                        if confirm_res {
                            let selected_ap = *ap_list_widget.selected_access_point();
                            assert!(!selected_ap.is_nil());

                            //Start sniffing for target devices
                            self.monitor.sniff_devices(selected_ap);

                            //Move onto selecting the target device
                            self.state = TargetSelectState::dev_select(selected_ap, &self.monitor);
                        } else {
                            *confirmation_widget_opt = None;
                        }
                    }
                } else {
                    //Ask for confirmation upon pressing enter
                    if let Event::Key(key) = event {
                        if key.kind == KeyEventKind::Press && key.code == KeyCode::Enter {
                            if !ap_list_widget.selected_access_point().is_nil() {
                                *confirmation_widget_opt = Some(ConfirmationWidget::new(
                                    Line::from(vec![
                                        "Do you want to select AP ".into(),
                                        ap_list_widget
                                            .selected_access_point()
                                            .to_hex_string()
                                            .bold(),
                                        " as the target access point?".into(),
                                    ])
                                    .into(),
                                ));
                            }
                            return;
                        }
                    }

                    ap_list_widget.handle_event(&self.monitor, event);
                }
            }

            TargetSelectState::DevSelect {
                target_ap_mac,
                dev_list_widget,
                confirmation_widget: confirmation_widget_opt,
            } => {
                //Handle access point select inputs
                if let Some(confirmation_widget) = confirmation_widget_opt {
                    if let Some(confirm_res) = confirmation_widget.handle_event(event) {
                        if confirm_res {
                            let selected_dev = *dev_list_widget.selected_device();
                            assert!(!selected_dev.is_nil());

                            //Invoke the callback
                            if let Some(cb) = self.callback.take() {
                                cb(*target_ap_mac, selected_dev);
                            }
                        } else {
                            *confirmation_widget_opt = None;
                        }
                    }
                } else {
                    //Ask for confirmation upon pressing enter
                    if let Event::Key(key) = event {
                        if key.kind == KeyEventKind::Press && key.code == KeyCode::Enter {
                            if !dev_list_widget.selected_device().is_nil() {
                                *confirmation_widget_opt = Some(ConfirmationWidget::new(
                                    Line::from(vec![
                                        "Do you want to select device ".into(),
                                        dev_list_widget.selected_device().to_hex_string().bold(),
                                        " as the target device?".into(),
                                    ])
                                    .into(),
                                ));
                            }
                            return;
                        }
                    }

                    dev_list_widget.handle_event(&self.monitor, event);
                }
            }
        }
    }
}

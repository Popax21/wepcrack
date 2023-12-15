use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{prelude::Rect, style::Stylize, text::Line, Frame};

use crate::{
    nl80211::{NL80211Connection, NL80211Wiphy},
    ui::{draw_ui_widgets, ConfirmationWidget, UIScene},
};

use super::{DeviceList, DeviceListWidget};

pub struct UIDeviceSelect {
    dev_list: DeviceList,
    dev_list_widget: DeviceListWidget,
    confirmation: Option<ConfirmationWidget<'static, DeviceList>>,
    callback: Option<Box<dyn FnOnce(NL80211Wiphy)>>,
}

impl UIDeviceSelect {
    #[allow(clippy::new_without_default)]
    pub fn new(
        nl80211_con: &NL80211Connection,
        callback: impl FnOnce(NL80211Wiphy) + 'static,
    ) -> UIDeviceSelect {
        //Query the device list
        let dev_list =
            DeviceList::query_list(nl80211_con).expect("failed to query the device list");

        UIDeviceSelect {
            dev_list_widget: DeviceListWidget::new(&dev_list),
            dev_list,
            confirmation: None,
            callback: Some(Box::new(callback)),
        }
    }
}

impl UIScene for UIDeviceSelect {
    fn should_quit(&self) -> bool {
        false
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) {
        if let Some(confirmation) = &mut self.confirmation {
            draw_ui_widgets(
                &mut [&mut self.dev_list_widget, confirmation],
                &self.dev_list,
                frame,
                area,
            );
        } else {
            draw_ui_widgets(
                &mut [&mut self.dev_list_widget],
                &self.dev_list,
                frame,
                area,
            );
        }
    }

    fn handle_event(&mut self, event: &Event) {
        //Handle confirmation
        if let Some(confirmation) = &mut self.confirmation {
            let Some(res) = confirmation.handle_event(event) else {
                return;
            };

            if res {
                //Invoke the callback
                if let Some(cb) = self.callback.take() {
                    cb(self
                        .dev_list_widget
                        .selected_device(&self.dev_list)
                        .unwrap()
                        .wiphy()
                        .clone());
                }
            } else {
                //User cancelled
                self.confirmation = None;
                return;
            }
        }

        //Handle the device selection
        if let Event::Key(key) = event {
            if key.kind == KeyEventKind::Press && key.code == KeyCode::Enter {
                let Some(dev) = self.dev_list_widget.selected_device(&self.dev_list) else {
                    return;
                };
                if !dev.is_suitable() {
                    return;
                }

                //Ask for confirmation
                self.confirmation = Some(ConfirmationWidget::new(Line::from(vec![
                    "Do you want to switch wiphy ".into(),
                    dev.name().to_owned().bold(),
                    " into monitor mode?".into(),
                ])));
                return;
            }
        }

        self.dev_list_widget.handle_event(&self.dev_list, event);
    }
}

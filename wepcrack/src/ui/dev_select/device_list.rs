use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{
    prelude::{Constraint, Direction, Layout, Rect},
    style::{Color, Stylize},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::ui::{draw_ui_widget_border, UIWidget};

use super::{Device, DeviceList};

pub(super) struct DeviceListWidget {
    selected_device_idx: usize,
}

impl DeviceListWidget {
    pub fn new(dev_list: &DeviceList) -> DeviceListWidget {
        DeviceListWidget {
            selected_device_idx: dev_list
                .devices()
                .iter()
                .position(Device::is_suitable)
                .unwrap_or_default(),
        }
    }
}

impl DeviceListWidget {
    pub fn handle_event(&mut self, dev_list: &DeviceList, event: &Event) {
        if let Event::Key(key) = event {
            if key.kind == KeyEventKind::Release {
                return;
            }

            //Handle device list selection
            let dir = match key.code {
                KeyCode::Up => -1isize,
                KeyCode::Down => 1isize,
                _ => return,
            };

            //Move up/down the list until we find a new suitable device
            let mut idx = self.selected_device_idx as isize;
            loop {
                //Move one up/down the list
                idx = (idx + dir).rem_euclid(dev_list.devices().len() as isize);

                //Check if we wrapped around to our original selection
                if idx == self.selected_device_idx as isize {
                    break;
                }

                //Check if we landed on a suitable device
                if dev_list.devices()[idx as usize].is_suitable() {
                    self.selected_device_idx = idx as usize;
                    break;
                }
            }
        }
    }

    pub fn selected_device<'a>(&self, dev_list: &'a DeviceList) -> Option<&'a Device> {
        let dev = &dev_list.devices()[self.selected_device_idx];
        if dev.is_suitable() {
            Some(dev)
        } else {
            None
        }
    }

    fn draw_device_list_entry_header(
        &self,
        device: &Device,
        frame: &mut Frame,
        area: Rect,
        is_selected: bool,
    ) {
        let [selected_area, name_area] = *Layout::new()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area)
        else {
            unreachable!();
        };

        // - selection highlight
        frame.render_widget(
            Paragraph::new(if device.is_suitable() {
                if is_selected { " > " } else { "   " }.into()
            } else {
                " x ".red()
            }),
            selected_area,
        );

        // - name
        frame.render_widget(
            Paragraph::new(device.name().bold().fg(if device.is_suitable() {
                Color::Cyan
            } else {
                Color::Red
            })),
            name_area,
        );
    }

    fn draw_device_list_entry(
        &self,
        device: &Device,
        frame: &mut Frame,
        area: Rect,
        is_selected: bool,
    ) {
        let [header_area, info_area] = *Layout::new()
            .constraints([Constraint::Length(1), Constraint::Min(0)])
            .split(area)
        else {
            unreachable!();
        };

        //Draw the header
        self.draw_device_list_entry_header(device, frame, header_area, is_selected);

        //Draw info
        let info_layout = Layout::new()
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(
                Layout::new()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Length(5), Constraint::Min(0)])
                    .split(info_area)[1],
            );

        // - interfaces
        let mut interfaces_line = vec!["interfaces: ".into()];
        for interf in device.interfaces() {
            if interfaces_line.len() > 1 {
                interfaces_line.push(", ".into());
            }
            interfaces_line.push(interf.name().bold());
        }
        if device.interfaces().is_empty() {
            interfaces_line.push("none".gray());
        }
        frame.render_widget(Paragraph::new(Line::from(interfaces_line)), info_layout[0]);

        // - RFKill
        frame.render_widget(
            Paragraph::new(Line::from({
                let mut line = Vec::<Span>::new();

                line.push("rfkill: ".into());
                if let Some(rfkill) = device.rfkill() {
                    line.push(rfkill.name().bold());
                    line.push(" (".into());

                    let (hwlock, swlock) = (rfkill.is_hard_locked(), rfkill.is_soft_locked());
                    if hwlock {
                        line.push("hwlock".red().bold());
                    }
                    if swlock {
                        if hwlock {
                            line.push(" ".into());
                        }
                        line.push("swlock".light_red().bold());
                    }
                    if !hwlock && !swlock {
                        line.push("unlocked".green().bold());
                    }

                    line.push(")".into());
                } else {
                    line.push("none".gray().bold());
                }

                line
            })),
            info_layout[1],
        );

        // - monitor mode
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                "monitor mode: ".into(),
                if device.supports_monitor_mode() {
                    "supported".green()
                } else {
                    "not supported".red()
                }
                .bold(),
            ])),
            info_layout[2],
        );
    }
}

impl UIWidget<'_> for DeviceListWidget {
    type SharedState = DeviceList;

    fn size(&self, dev_list: &DeviceList) -> u16 {
        2 + 4 * dev_list.devices().len() as u16
    }

    fn draw(&mut self, dev_list: &DeviceList, frame: &mut Frame, area: Rect) {
        draw_ui_widget_border("Device List", frame, area);

        //Calculate the layout
        let layout = Layout::new()
            .margin(1)
            .constraints(
                dev_list
                    .devices()
                    .iter()
                    .map(|_| Constraint::Length(3))
                    .collect::<Vec<_>>(),
            )
            .split(area);

        //Draw the device list
        for (idx, dev) in dev_list.devices().iter().enumerate() {
            self.draw_device_list_entry(dev, frame, layout[idx], idx == self.selected_device_idx);
        }
    }
}

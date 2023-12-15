use crossterm::event::{Event, KeyCode, KeyEventKind};
use ieee80211::MacAddress;
use ratatui::{
    prelude::{Margin, Rect},
    style::{Color, Style, Stylize},
    symbols::scrollbar,
    text::Line,
    widgets::{
        HighlightSpacing, List, ListItem, ListState, Scrollbar, ScrollbarOrientation,
        ScrollbarState,
    },
    Frame,
};

use crate::ui::{draw_ui_widget_border, UIWidget};

use super::TargetMonitor;

pub struct UITargetDeviceList {
    selected_dev_mac: MacAddress,
    list_scroll: usize,
}

impl UITargetDeviceList {
    const LIST_SIZE: usize = 16;

    pub fn new(_target_mon: &TargetMonitor) -> UITargetDeviceList {
        UITargetDeviceList {
            selected_dev_mac: MacAddress::default(),
            list_scroll: 0,
        }
    }

    fn update_list_scroll(&mut self, dev_idx: usize) {
        if dev_idx < self.list_scroll {
            self.list_scroll = dev_idx;
        } else if dev_idx >= self.list_scroll + Self::LIST_SIZE {
            self.list_scroll = dev_idx - Self::LIST_SIZE + 1;
        }
    }

    pub fn handle_event(&mut self, target_mon: &TargetMonitor, event: &Event) {
        let Event::Key(event) = event else {
            return;
        };

        if event.kind == KeyEventKind::Release {
            return;
        }

        //Handle scrolling up/down the list
        let scroll_dir = match event.code {
            KeyCode::Up => -1isize,
            KeyCode::Down => 1isize,
            KeyCode::PageUp => -(Self::LIST_SIZE as isize),
            KeyCode::PageDown => Self::LIST_SIZE as isize,
            _ => return,
        };

        //Update the selected device
        let mut devs = target_mon.get_sniffed_devices();

        if devs.is_empty() {
            return;
        }

        devs.sort_by_key(|dev| -dev.strength_dbm());

        let mut dev_idx = devs
            .iter()
            .position(|ap| ap.mac_address() == &self.selected_dev_mac)
            .unwrap_or(0);

        dev_idx = (dev_idx as isize + scroll_dir)
            .max(0)
            .min(devs.len() as isize - 1) as usize;

        self.selected_dev_mac = *devs[dev_idx].mac_address();

        //Update the list scroll amount
        self.update_list_scroll(dev_idx);
    }

    pub const fn selected_device(&self) -> &MacAddress {
        &self.selected_dev_mac
    }
}

impl UIWidget<'_> for UITargetDeviceList {
    type SharedState = TargetMonitor;

    fn size(&self, _: &TargetMonitor) -> u16 {
        Self::LIST_SIZE as u16 + 2
    }

    fn draw(&mut self, target_mon: &TargetMonitor, frame: &mut Frame, area: Rect) {
        draw_ui_widget_border("Target Devices", frame, area);
        let area = area.inner(&Margin::new(1, 1));

        //Find the currently selected target device in the list
        let mut devs = target_mon.get_sniffed_devices();
        devs.sort_by_key(|ap| -ap.strength_dbm());

        if self.selected_dev_mac.is_nil() && !devs.is_empty() {
            self.selected_dev_mac = *devs[0].mac_address();
        }

        let selected_dev_idx = devs
            .iter()
            .position(|ap| ap.mac_address() == &self.selected_dev_mac)
            .unwrap_or(0);

        //Update the list scroll amount
        self.update_list_scroll(selected_dev_idx);

        //Draw the target device list
        let list = devs
            .iter()
            .skip(self.list_scroll)
            .take(Self::LIST_SIZE)
            .map(|ap| {
                ListItem::new(Line::from(vec![
                    ap.mac_address().to_hex_string().bold(),
                    " @ ".dark_gray(),
                    format!("{:3}", ap.strength_dbm()).into(),
                    "dBm".dark_gray(),
                ]))
            })
            .collect::<Vec<_>>();

        frame.render_stateful_widget(
            List::new(list)
                .highlight_symbol("> ")
                .highlight_spacing(HighlightSpacing::Always)
                .highlight_style(Style::new().fg(Color::Cyan).bold()),
            area,
            &mut ListState::default().with_selected(Some(selected_dev_idx - self.list_scroll)),
        );

        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight).symbols(scrollbar::VERTICAL),
            area,
            &mut ScrollbarState::new(devs.len()).position(self.list_scroll),
        );
    }
}

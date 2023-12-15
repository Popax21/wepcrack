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

pub struct UIAccessPointList {
    selected_ap_mac: MacAddress,
    list_scroll: usize,
}

impl UIAccessPointList {
    const LIST_SIZE: usize = 16;

    pub fn new(_target_mon: &TargetMonitor) -> UIAccessPointList {
        UIAccessPointList {
            selected_ap_mac: MacAddress::default(),
            list_scroll: 0,
        }
    }

    fn update_list_scroll(&mut self, ap_idx: usize) {
        if ap_idx < self.list_scroll {
            self.list_scroll = ap_idx;
        } else if ap_idx >= self.list_scroll + Self::LIST_SIZE {
            self.list_scroll = ap_idx - Self::LIST_SIZE + 1;
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

        //Update the selected AP
        let mut aps = target_mon.get_sniffed_aps();

        if aps.is_empty() {
            return;
        }

        aps.sort_by_key(|ap| -ap.strength_dbm());

        let mut ap_idx = aps
            .iter()
            .position(|ap| ap.mac_address() == &self.selected_ap_mac)
            .unwrap_or(0);

        ap_idx = (ap_idx as isize + scroll_dir)
            .max(0)
            .min(aps.len() as isize - 1) as usize;

        self.selected_ap_mac = *aps[ap_idx].mac_address();

        //Update the list scroll amount
        self.update_list_scroll(ap_idx);
    }

    pub const fn selected_access_point(&self) -> &MacAddress {
        &self.selected_ap_mac
    }
}

impl UIWidget<'_> for UIAccessPointList {
    type SharedState = TargetMonitor;

    fn size(&self, _: &TargetMonitor) -> u16 {
        Self::LIST_SIZE as u16 + 2
    }

    fn draw(&mut self, target_mon: &TargetMonitor, frame: &mut Frame, area: Rect) {
        draw_ui_widget_border("Access Points", frame, area);
        let area = area.inner(&Margin::new(1, 1));

        //Find the currently selected access point in the list
        let mut aps = target_mon.get_sniffed_aps();
        aps.sort_by_key(|ap| -ap.strength_dbm());

        if self.selected_ap_mac.is_nil() && !aps.is_empty() {
            self.selected_ap_mac = *aps[0].mac_address();
        }

        let selected_ap_idx = aps
            .iter()
            .position(|ap| ap.mac_address() == &self.selected_ap_mac)
            .unwrap_or(0);

        //Update the list scroll amount
        self.update_list_scroll(selected_ap_idx);

        //Draw the access point list
        let list = aps
            .iter()
            .skip(self.list_scroll)
            .take(Self::LIST_SIZE)
            .map(|ap| {
                let mut line = Vec::new();

                line.push(ap.mac_address().to_hex_string().bold());
                line.push(" @ ".dark_gray());
                line.push(format!("{:3}", ap.strength_dbm()).into());
                line.push("dBm".dark_gray());

                if let Some(ssid) = ap.ssid() {
                    line.push(" [".dark_gray());
                    line.push(ssid.into());
                    line.push("]".dark_gray());
                }

                ListItem::new(Line::from(line))
            })
            .collect::<Vec<_>>();

        frame.render_stateful_widget(
            List::new(list)
                .highlight_symbol("> ")
                .highlight_spacing(HighlightSpacing::Always)
                .highlight_style(Style::new().fg(Color::Cyan).bold()),
            area,
            &mut ListState::default().with_selected(Some(selected_ap_idx - self.list_scroll)),
        );

        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight).symbols(scrollbar::VERTICAL),
            area,
            &mut ScrollbarState::new(aps.len()).position(self.list_scroll),
        );
    }
}

use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{
    prelude::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Style, Stylize},
    symbols::scrollbar,
    text::Line,
    widgets::{
        HighlightSpacing, List, ListItem, ListState, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState,
    },
    Frame,
};

use crate::{
    nl80211::{NL80211Channel, NL80211ChannelBand},
    ui::{draw_ui_widget_border, UIWidget},
};

use super::TargetMonitor;

pub struct UIChannelSelect {
    selected_channel_idx: usize,
    list_scroll: usize,
}

impl UIChannelSelect {
    const LIST_SIZE: usize = 16;

    pub fn new() -> UIChannelSelect {
        UIChannelSelect {
            selected_channel_idx: 0,
            list_scroll: 0,
        }
    }

    pub fn draw_channel_select(&self, target_mon: &TargetMonitor, frame: &mut Frame, area: Rect) {
        draw_ui_widget_border("Channel Selection", frame, area);
        let area = area.inner(&Margin::new(1, 1));

        //Draw the channel list
        let list = target_mon
            .monitor()
            .channels()
            .iter()
            .skip(self.list_scroll)
            .take(Self::LIST_SIZE)
            .map(|channel| ListItem::new(channel.to_string()))
            .collect::<Vec<_>>();

        frame.render_stateful_widget(
            List::new(list)
                .highlight_symbol("> ")
                .highlight_spacing(HighlightSpacing::Always)
                .highlight_style(Style::new().fg(Color::Cyan).bold()),
            area,
            &mut ListState::default()
                .with_selected(Some(self.selected_channel_idx - self.list_scroll)),
        );

        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight).symbols(scrollbar::VERTICAL),
            area,
            &mut ScrollbarState::new(target_mon.monitor().channels().len())
                .position(self.list_scroll),
        );
    }

    pub fn draw_channel_info(&self, target_mon: &TargetMonitor, frame: &mut Frame, area: Rect) {
        draw_ui_widget_border("Channel Information", frame, area);
        let channel = &target_mon.monitor().channels()[self.selected_channel_idx];

        //Calculate the layout
        let layout = Layout::new()
            .constraints(
                std::iter::repeat(Constraint::Length(1))
                    .take(Self::LIST_SIZE)
                    .collect::<Vec<_>>(),
            )
            .horizontal_margin(2)
            .vertical_margin(1)
            .split(area);

        //Draw channel information
        let mut next_stat_idx = 0;
        let mut draw_info = |name: &str, val: &str| {
            frame.render_widget(
                Paragraph::new(Line::from(vec![name.bold(), ": ".bold(), val.into()])),
                layout[next_stat_idx],
            );
            next_stat_idx += 1;
        };

        // - channel indices
        match channel {
            NL80211Channel::Channel20NoHT { channel } | NL80211Channel::ChannelHT20 { channel } => {
                draw_info("channel index", &channel.to_string());
            }
            NL80211Channel::ChannelHT40 {
                main_channel,
                aux_channel,
            }
            | NL80211Channel::ChannelVHT80 {
                main_channel,
                aux_channel,
            }
            | NL80211Channel::ChannelVHT160 {
                main_channel,
                aux_channel,
            } => {
                draw_info("main channel index", &main_channel.to_string());
                draw_info("aux channel index", &aux_channel.to_string());
            }
        }

        // - frequency info
        let freq_range = channel.freq_range();
        draw_info(
            "frequency",
            &format!(
                "{:5.3}Ghz ({:5.3}-{:5.3}GHz)",
                channel.frequency() as f64 / 1000.,
                freq_range.start as f64 / 1000.,
                (freq_range.end - 1) as f64 / 1000.
            ),
        );

        draw_info("bandwidth", &format!("{}MHz", channel.width().bandwidth()));

        draw_info(
            "frequency band",
            match channel.band() {
                NL80211ChannelBand::Band2400Mhz => "2.4GHz",
                NL80211ChannelBand::Band5Ghz => "5Ghz",
            },
        );

        // - channel type
        match channel {
            NL80211Channel::Channel20NoHT { channel: _ } => draw_info("type", "old / no HT20"),
            NL80211Channel::ChannelHT20 { channel: _ } => draw_info("type", "HT20"),
            NL80211Channel::ChannelHT40 {
                main_channel,
                aux_channel,
            } => draw_info(
                "type",
                if aux_channel > main_channel {
                    "HT40+"
                } else {
                    "HT40-"
                },
            ),
            NL80211Channel::ChannelVHT80 {
                main_channel,
                aux_channel,
            } => draw_info(
                "type",
                if aux_channel > main_channel {
                    "VHT80+"
                } else {
                    "VHT80-"
                },
            ),
            NL80211Channel::ChannelVHT160 {
                main_channel,
                aux_channel,
            } => draw_info(
                "type",
                if aux_channel > main_channel {
                    "VHT160+"
                } else {
                    "VHT160-"
                },
            ),
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

        self.selected_channel_idx = (self.selected_channel_idx as isize + scroll_dir)
            .max(0)
            .min(target_mon.monitor().channels().len() as isize - 1)
            as usize;

        if self.selected_channel_idx < self.list_scroll {
            self.list_scroll = self.selected_channel_idx;
        } else if self.selected_channel_idx >= self.list_scroll + Self::LIST_SIZE {
            self.list_scroll = self.selected_channel_idx - Self::LIST_SIZE + 1;
        }
    }
}

impl UIWidget<'_> for UIChannelSelect {
    type SharedState = TargetMonitor;

    fn size(&self, _: &TargetMonitor) -> u16 {
        Self::LIST_SIZE as u16
    }

    fn draw(&mut self, target_mon: &TargetMonitor, frame: &mut Frame, area: Rect) {
        let layout: std::rc::Rc<[Rect]> = Layout::new()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
            .split(area);

        self.draw_channel_select(target_mon, frame, layout[0]);
        self.draw_channel_info(target_mon, frame, layout[1]);
    }
}

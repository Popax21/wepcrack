use std::{borrow::Cow, collections::VecDeque};

use ratatui::{
    prelude::{Constraint, Margin, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{List, ListItem},
    Frame,
};

use crate::ui::{draw_ui_widget_border, UIWidget};

use super::DevManager;

#[allow(unused)]
pub enum LogLevel {
    Info,
    Warning,
    Error,
}

pub struct LogLine(pub LogLevel, pub Cow<'static, str>);

impl<'a> From<&'a LogLine> for Span<'a> {
    fn from(value: &'a LogLine) -> Self {
        Span::styled(
            value.1.as_ref(),
            match value.0 {
                LogLevel::Info => Style::default(),
                LogLevel::Warning => Style::new().fg(Color::Yellow),
                LogLevel::Error => Style::new().fg(Color::Red),
            },
        )
    }
}

pub(super) struct LogBuffer {
    max_lines: usize,
    lines: VecDeque<LogLine>,
}

impl LogBuffer {
    pub fn new(max_lines: usize) -> LogBuffer {
        LogBuffer {
            max_lines,
            lines: VecDeque::with_capacity(max_lines),
        }
    }

    pub const fn max_lines(&self) -> usize {
        self.max_lines
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn clear(&mut self) {
        self.lines.clear();
    }

    pub fn add_line(&mut self, line: LogLine) {
        while self.lines.len() >= self.max_lines {
            self.lines.pop_front();
        }
        self.lines.push_back(line);
    }
}

pub(super) struct LogWidget;

impl UIWidget<'_> for LogWidget {
    type SharedState = DevManager;

    fn size(&self, dev_manager: &DevManager) -> Constraint {
        Constraint::Length(dev_manager.log_buffer().max_lines() as u16 + 2)
    }

    fn draw(&mut self, dev_manager: &DevManager, frame: &mut Frame, area: Rect) {
        draw_ui_widget_border("Setup Log", frame, area);

        frame.render_widget(
            List::new(
                dev_manager
                    .log_buffer()
                    .lines
                    .iter()
                    .map(|log_line| {
                        ListItem::new({
                            let mut line = Line::from("> ");
                            line.spans.push(Span::from(log_line));
                            line
                        })
                    })
                    .collect::<Vec<ListItem>>(),
            ),
            area.inner(&Margin::new(1, 1)),
        )
    }
}

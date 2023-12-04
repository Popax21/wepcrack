use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Stylize,
    text::Line,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::{keycracker::WepKeyCracker, ui::UIWidget};

pub(crate) struct SampleStatsWidget<'a>(&'a WepKeyCracker);

impl SampleStatsWidget<'_> {
    pub fn new(cracker: &WepKeyCracker) -> SampleStatsWidget<'_> {
        SampleStatsWidget(cracker)
    }
}

impl UIWidget for SampleStatsWidget<'_> {
    fn size(&self) -> Constraint {
        Constraint::Length(3)
    }

    fn draw(&self, frame: &mut Frame, area: Rect) {
        let layout = Layout::default()
            .constraints([Constraint::Length(1)])
            .margin(1)
            .split(area);

        //Draw the border block
        frame.render_widget(
            Block::default()
                .borders(Borders::all())
                .title("Sample Stats"),
            area,
        );

        //Draw the stats text
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                "#samples: ".bold(),
                format!("{:8}", self.0.num_samples()).into(),
            ])),
            layout[0],
        );
    }
}

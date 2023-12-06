use ratatui::{
    prelude::{Constraint, Direction, Layout, Rect},
    style::Stylize,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::{
    keycracker::{KeyBytePrediction, KeyTester},
    wep::WepKey,
};

use super::{KeyCracker, KeyCrackerWidget};

pub(crate) struct CandidateKeyTestingWidget;

impl CandidateKeyTestingWidget {
    pub fn new() -> CandidateKeyTestingWidget {
        CandidateKeyTestingWidget
    }

    fn draw_info(&self, tester: &KeyTester, frame: &mut Frame, area: Rect) {
        let layout = Layout::new()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(15), Constraint::Min(0)])
            .split(area);

        frame.render_widget(Paragraph::new("maybe WEP40:".bold()), layout[0]);
        frame.render_widget(
            Paragraph::new(match tester.is_maybe_wep40() {
                true => "yes".green(),
                false => "no".red(),
            }),
            layout[1],
        );
    }

    fn draw_candidate_key(&self, tester: &KeyTester, frame: &mut Frame, area: Rect) {
        //Construct the line
        let key = tester.current_key();
        let mut line = Vec::<Span<'_>>::new();
        for (i, keybyte) in key.iter().enumerate() {
            if i > 0 {
                line.push(" ".into());
            }

            line.push(match tester.key_predictions()[i] {
                KeyBytePrediction::Normal { sigma: _ } => {
                    format!("{:02x}", keybyte).on_light_magenta()
                }
                KeyBytePrediction::Strong => format!("{:02x}", keybyte).on_light_cyan(),
            });
        }

        //Draw the line
        let layout = Layout::new()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(20), Constraint::Min(0)])
            .split(area);

        frame.render_widget(Paragraph::new("current candidate:".bold()), layout[0]);
        frame.render_widget(Paragraph::new(Line::from(line)), layout[1]);
    }

    fn draw_l_indices(&self, tester: &KeyTester, frame: &mut Frame, area: Rect) {
        //Construct the line
        let mut line = Vec::<Span<'_>>::new();
        for i in 0..WepKey::LEN_104 {
            if i > 0 {
                line.push(" ".into());
            }

            line.push(match tester.key_predictions()[i] {
                KeyBytePrediction::Normal { sigma: _ } => "--".on_light_magenta(),
                KeyBytePrediction::Strong => {
                    format!("{:2}", tester.current_l_indices()[i]).on_light_cyan()
                }
            });
        }

        //Draw the line
        let layout = Layout::new()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(20), Constraint::Min(0)])
            .split(area);

        frame.render_widget(Paragraph::new("current l-indices:".bold()), layout[0]);
        frame.render_widget(Paragraph::new(Line::from(line)), layout[1]);
    }
}

impl KeyCrackerWidget for CandidateKeyTestingWidget {
    fn size(&self) -> Constraint {
        Constraint::Length(5)
    }

    fn draw(&mut self, cracker: &KeyCracker, frame: &mut Frame, area: Rect) {
        let tester = cracker.key_tester().unwrap();

        //Calculate the layout
        let [info_layout, cand_key_layout, l_idxs_layout] = Layout::default()
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .margin(1)
            .split(area)[..]
        else {
            unreachable!();
        };

        //Draw the border block
        frame.render_widget(
            Block::default()
                .borders(Borders::all())
                .title("Candidate Key Testing"),
            area,
        );

        //Draw general info
        self.draw_info(tester, frame, info_layout);

        //Draw the current candidate key
        self.draw_candidate_key(tester, frame, cand_key_layout);

        //Draw the l indices
        self.draw_l_indices(tester, frame, l_idxs_layout);
    }
}

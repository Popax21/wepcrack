use std::time::Instant;

use ratatui::{
    layout::{Constraint, Layout, Rect},
    prelude::Direction,
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{Block, Borders, Gauge, LineGauge, Paragraph},
    Frame,
};

use super::{KeyCracker, KeyCrackerPhase, KeyCrackerWidget};

pub(crate) struct OverviewWidget {
    start_time: Instant,
    end_time: Option<Instant>,

    last_draw: Instant,
    last_draw_samples: usize,
    smoothed_sample_rate: f64,
}

impl OverviewWidget {
    pub fn new() -> OverviewWidget {
        OverviewWidget {
            start_time: Instant::now(),
            end_time: None,

            last_draw: Instant::now(),
            last_draw_samples: 0,
            smoothed_sample_rate: 0.,
        }
    }

    fn draw_sample_stats(&mut self, cracker: &KeyCracker, frame: &mut Frame, area: Rect) {
        //Update the sample rate
        let time_delta = self.last_draw.elapsed();
        self.last_draw = Instant::now();

        let sample_rate = (cracker.key_predictor().num_samples() - self.last_draw_samples) as f64
            / time_delta.as_secs_f64();
        self.last_draw_samples = cracker.key_predictor().num_samples();

        const SAMPLE_RATE_BLEED: f64 = 0.9;
        self.smoothed_sample_rate =
            self.smoothed_sample_rate * SAMPLE_RATE_BLEED + sample_rate * (1. - SAMPLE_RATE_BLEED);

        //Calculate the layout
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(20),
                Constraint::Length(20),
                Constraint::Min(0),
            ])
            .split(area);

        // - number of samples
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                "#samples: ".bold(),
                format!("{}", cracker.key_predictor().num_samples()).into(),
            ])),
            layout[0],
        );

        // - sample rate
        //Only show it when collecting samples
        if let KeyCrackerPhase::SampleCollection = cracker.phase() {
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    "samples/s: ".bold(),
                    format!("{:10.4}", self.smoothed_sample_rate).into(),
                ])),
                layout[1],
            );
        }
    }

    fn draw_test_buf_stats(&self, cracker: &KeyCracker, frame: &mut Frame, area: Rect) {
        let layout: std::rc::Rc<[Rect]> = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(40),
                Constraint::Length(25),
                Constraint::Min(0),
            ])
            .split(area);

        // - utilization
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                "test sample buffer: ".bold(),
                format!(
                    "{:5} / {:5}",
                    cracker.test_sample_buf().num_samples(),
                    cracker.settings().num_test_samples
                )
                .into(),
            ])),
            layout[0],
        );

        // - gauge
        frame.render_widget(
            LineGauge::default()
                .gauge_style(Style::new().light_cyan())
                .ratio(
                    cracker.test_sample_buf().num_samples() as f64
                        / cracker.settings().num_test_samples as f64,
                ),
            layout[1],
        );
    }

    fn draw_key_tester_stats(&self, cracker: &KeyCracker, frame: &mut Frame, area: Rect) {
        let tester = cracker.key_tester().unwrap();

        let mut key_idx = tester.current_key_index();
        if cracker.phase() == KeyCrackerPhase::FinishedSuccess {
            key_idx += 1;
        }

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                "tested candidate keys: ".bold(),
                format!("{key_idx} / {}", tester.num_keys()).into(),
            ])),
            area,
        );
    }
}

impl KeyCrackerWidget for OverviewWidget {
    fn size(&self) -> Constraint {
        Constraint::Length(2 + 1 + 1 + 1 + 1 + 2)
    }

    fn draw(&mut self, cracker: &KeyCracker, frame: &mut Frame, area: Rect) {
        //Calculate the layout
        let [runtime_layout, sample_stats_layout, test_layout, _, progbar_layout] =
            Layout::default()
                .constraints([
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Max(1),
                    Constraint::Length(2),
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
                .title("Overview")
                .style(match cracker.phase() {
                    KeyCrackerPhase::FinishedSuccess => Style::new().bg(Color::LightGreen),
                    KeyCrackerPhase::FinishedFailure => Style::new().bg(Color::LightRed),
                    _ => Style::default(),
                }),
            area,
        );

        //Draw the runtime text
        if !cracker.is_running() && self.end_time.is_none() {
            self.end_time = Some(Instant::now());
        }

        let runtime = match cracker.is_running() {
            true => self.start_time.elapsed(),
            false => self.end_time.unwrap() - self.start_time,
        };

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                "runtime: ".bold(),
                format!(
                    "{:2}min {:2}sec",
                    runtime.as_secs() / 60,
                    runtime.as_secs() % 60
                )
                .into(),
            ])),
            runtime_layout,
        );

        //Draw the sample stats text
        self.draw_sample_stats(cracker, frame, sample_stats_layout);

        //Draw the test sample buffer / key tester statistics
        if cracker.phase() < KeyCrackerPhase::CandidateKeyTesting {
            self.draw_test_buf_stats(cracker, frame, test_layout);
        } else {
            self.draw_key_tester_stats(cracker, frame, test_layout);
        }

        //Draw the progress gauge
        if cracker.is_running() {
            frame.render_widget(
                Gauge::default()
                    .gauge_style(Style::new().blue())
                    .block(Block::default().title(match cracker.phase() {
                        KeyCrackerPhase::SampleCollection => {
                            "Collecting samples for sigma sum prediction..."
                        }
                        KeyCrackerPhase::CandidateKeyTesting => "Testing candidate keys...",
                        _ => unreachable!(),
                    }))
                    .ratio(cracker.progress()),
                progbar_layout,
            );
        } else if let Some(cracked_key) = cracker.cracked_key() {
            let layout = Layout::default()
                .constraints([Constraint::Length(1), Constraint::Length(1)])
                .split(progbar_layout);

            frame.render_widget(
                Paragraph::new("Done - Found WEP Key! \\(^-^)/".bold()),
                layout[0],
            );
            frame.render_widget(
                Paragraph::new(Line::from(match cracked_key {
                    crate::wep::WepKey::Wep40Key(key) => {
                        vec!["WEP-40 key: ".bold(), hex::encode(key).into()]
                    }
                    crate::wep::WepKey::Wep104Key(key) => {
                        vec!["WEP-104 key: ".bold(), hex::encode(key).into()]
                    }
                })),
                layout[1],
            );
        } else {
            frame.render_widget(
                Paragraph::new("").block(
                    Block::default()
                        .title("Done - Didn't find WEP Key :(")
                        .bold(),
                ),
                progbar_layout,
            );
        }
    }
}

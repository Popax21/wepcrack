use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Stylize,
    text::{Line, Span},
    widgets::{List, ListItem},
    Frame,
};

use crate::{
    keycracker::KeyBytePrediction,
    ui::{draw_ui_widget_border, UIWidget},
    wep::WepKey,
};

use super::KeyCracker;

pub(super) struct SigmaInfoWidget;

impl SigmaInfoWidget {
    pub fn new() -> SigmaInfoWidget {
        SigmaInfoWidget
    }
}

impl UIWidget<'_> for SigmaInfoWidget {
    type SharedState = KeyCracker;

    fn size(&self, _cracker: &KeyCracker) -> u16 {
        2 + WepKey::LEN_104 as u16
    }

    fn draw(&mut self, cracker: &KeyCracker, frame: &mut Frame, area: Rect) {
        draw_ui_widget_border("Sigma Sums", frame, area);

        //Calculate the layout
        let layout = Layout::default()
            .margin(1)
            .constraints([Constraint::Length(WepKey::LEN_104 as u16)])
            .split(area);

        //Draw the list
        let mut sigma_list = Vec::<ListItem>::new();

        for i in 0..WepKey::LEN_104 {
            //Get key byte info
            let info = cracker.key_predictor().key_byte_info(i);

            //Construct the info line
            let mut info_line = Vec::<Span<'_>>::new();

            info_line.extend_from_slice(&[
                "σ".cyan().bold(),
                "[".dark_gray(),
                format!("{i:2}").into(),
                "]".dark_gray(),
                ":".into(),
            ]);

            // - probabilities
            info_line.extend_from_slice(&[
                " candidate=".dark_gray(),
                format!("{:02x}", info.candidate_sigma).into(),
                " p_candidate=".dark_gray(),
                format!("{:.8}", info.p_candidate).into(),
                " p_correct=".dark_gray(),
                format!("{:.8}", info.p_correct).into(),
                " p_equal=".dark_gray(),
            ]);

            // - errors
            info_line.extend_from_slice(&[
                format!("{:.8}", info.p_equal).into(),
                " err_normal=".dark_gray(),
                format!("{:1.9}", info.err_normal).into(),
                " err_strong=".dark_gray(),
                format!("{:1.9}", info.err_strong).into(),
            ]);

            // - prediction
            let prediction = info.prediction();
            let prediction_score = info.prediction_score();
            info_line.extend([
                " pred: ".dark_gray(),
                match prediction {
                    KeyBytePrediction::Normal { sigma: _ } => "normal".magenta(),
                    KeyBytePrediction::Strong => "strong".cyan(),
                }
                .bold(),
                format!(" {:7.3}%", prediction_score * 100.).into(),
            ]);

            //Create the list item
            let info_list_item = ListItem::new(Line::from(info_line));

            //Change the background color for predictions past the threshold
            let info_list_item = if prediction_score
                >= if matches!(prediction, KeyBytePrediction::Normal { sigma: _ }) {
                    cracker.settings().key_predictor_normal_threshold
                } else {
                    cracker.settings().key_predictor_strong_threshold
                } {
                match prediction {
                    KeyBytePrediction::Normal { sigma: _ } => info_list_item.on_light_magenta(),
                    KeyBytePrediction::Strong => info_list_item.on_light_cyan(),
                }
            } else {
                info_list_item
            };

            sigma_list.push(info_list_item);
        }

        frame.render_widget(List::new(sigma_list), layout[0]);
    }
}

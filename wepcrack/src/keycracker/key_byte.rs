use std::sync::OnceLock;

use crate::wep::WepKey;

#[derive(Default, Debug, Clone, Copy)]
pub struct KeyBytePredictionInfo {
    pub candidate_sigma: u8,

    pub p_candidate: f64,
    pub p_correct: f64,
    pub p_equal: f64,

    pub err_strong: f64,
    pub err_normal: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyBytePrediction {
    Normal { sigma: u8 },
    Strong,
}

impl KeyBytePredictionInfo {
    fn calc_p_correct() -> [f64; WepKey::LEN_104] {
        //Calculate p_correct for all key bytes
        let mut p_correct = [0f64; WepKey::LEN_104];

        fn p_nopick_i(opts: i32) -> f64 {
            1. - opts as f64 / 256.
        }

        let p_nopick: f64 = p_nopick_i(1);
        let p_nopick_ks: f64 = p_nopick.powi(254);

        let mut q_i_accum: f64 = 1.;
        for (i, p_correct) in p_correct.iter_mut().enumerate() {
            let q_i = q_i_accum * p_nopick_i(i as i32);
            q_i_accum *= p_nopick * p_nopick_i(i as i32 + 1);

            *p_correct =
                q_i * p_nopick_ks * 2. / 256. + (1. - q_i * p_nopick_ks) * 254. / (256. * 255.);
        }

        p_correct
    }

    pub fn from_sigma_votes(
        key_idx: usize,
        votes: &[usize; 256],
        total_votes: usize,
    ) -> KeyBytePredictionInfo {
        static P_CORRECT: OnceLock<[f64; WepKey::LEN_104]> = OnceLock::new();
        let p_correct = P_CORRECT.get_or_init(KeyBytePredictionInfo::calc_p_correct);

        //Find the index of the candidate sigma (= the one with the most votes)
        let candidate_sigma = votes
            .iter()
            .enumerate()
            .max_by(|(_, v1), (_, v2)| v1.cmp(v2))
            .unwrap()
            .0;

        //Calculate err_strong and err_weak
        let p_equal = 1f64 / 256f64;
        let p_correct = p_correct[key_idx];
        let p_wrong = (1f64 - p_correct) / 255f64;

        let mut err_strong = 0f64;
        let mut err_normal = 0f64;
        for (sigma, &votes) in votes.iter().enumerate() {
            let frac = votes as f64 / total_votes as f64;

            err_strong += (frac - p_equal) * (frac - p_equal);

            if sigma == candidate_sigma {
                err_normal += (frac - p_correct) * (frac - p_correct);
            } else {
                err_normal += (frac - p_wrong) * (frac - p_wrong);
            }
        }

        KeyBytePredictionInfo {
            candidate_sigma: candidate_sigma as u8,

            p_candidate: votes[candidate_sigma] as f64 / total_votes as f64,
            p_correct,
            p_equal,

            err_strong,
            err_normal,
        }
    }

    pub fn prediction(&self) -> KeyBytePrediction {
        if self.err_normal < self.err_strong {
            KeyBytePrediction::Normal {
                sigma: self.candidate_sigma,
            }
        } else {
            KeyBytePrediction::Strong
        }
    }

    pub fn prediction_score(&self) -> f64 {
        if self.err_normal < self.err_strong {
            (self.err_strong - self.err_normal) / self.err_normal
        } else {
            (self.err_normal - self.err_strong) / self.err_strong
        }
    }
}

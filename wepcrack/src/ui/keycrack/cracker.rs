use crate::{
    keycracker::{
        KeyBytePrediction, KeyBytePredictionInfo, KeyPredictor, KeystreamSample, TestSampleBuffer,
    },
    wep::WepKey,
};

#[derive(Debug, Clone, Copy)]
pub struct KeyCrackerSettings {
    //Sample collection settings
    pub key_predictor_threshold: f64,

    //Test buffer settings
    pub num_test_samples: usize,
    pub test_sample_period: usize,
    pub test_sample_threshold: f64,
}

pub type KeyCrackerSampleProvider = dyn FnMut() -> KeystreamSample + Send + Sync;

#[derive(Debug, Clone, Copy)]
pub(crate) enum KeyCrackerPhase {
    SampleCollection {
        delay_timer: usize,
    },
    CandidateKeyTesting {
        key_predictions: [KeyBytePrediction; WepKey::LEN_104],
        strong_opt_idxs: [usize; WepKey::LEN_104],
    },
    FinishedSuccess,
    FinishedFailure,
}

pub(crate) struct KeyCracker<'a> {
    phase: KeyCrackerPhase,

    pub settings: KeyCrackerSettings,
    pub sample_provider: &'a mut KeyCrackerSampleProvider,

    pub key_predictor: KeyPredictor,
    pub test_sample_buf: TestSampleBuffer,
}

impl KeyCracker<'_> {
    pub fn new(
        settings: KeyCrackerSettings,
        sample_provider: &mut KeyCrackerSampleProvider,
    ) -> KeyCracker<'_> {
        KeyCracker {
            phase: KeyCrackerPhase::SampleCollection { delay_timer: 0 },

            settings,
            sample_provider,

            key_predictor: KeyPredictor::new(),
            test_sample_buf: TestSampleBuffer::new(
                settings.num_test_samples,
                settings.test_sample_period,
                settings.test_sample_threshold,
            ),
        }
    }

    pub const fn phase(&self) -> KeyCrackerPhase {
        self.phase
    }

    pub const fn is_running(&self) -> bool {
        match self.phase {
            KeyCrackerPhase::FinishedSuccess | KeyCrackerPhase::FinishedFailure => false,
            _ => true,
        }
    }

    pub fn progress(&self) -> f64 {
        match &self.phase {
            KeyCrackerPhase::SampleCollection { delay_timer: _ } => {
                //Aggregate progress of all key bytes towards the threshold
                self.key_predictor
                    .key_byte_infos()
                    .iter()
                    .map(|info| {
                        (info.prediction_score() / self.settings.key_predictor_threshold).min(1.)
                    })
                    .sum::<f64>()
                    / self.key_predictor.key_byte_infos().len() as f64
            }
            KeyCrackerPhase::CandidateKeyTesting {
                key_predictions,
                strong_opt_idxs,
            } => {
                //Determine the number of candidate keys in addition to the current candidate key index
                let mut candidate_idx = 0;
                let mut num_candidates = 1;

                for i in 0..WepKey::LEN_104 {
                    if key_predictions[i] == KeyBytePrediction::Strong {
                        let num_opts = KeyBytePredictionInfo::num_strong_options(i);
                        candidate_idx = (candidate_idx * num_opts) + strong_opt_idxs[i];
                        num_candidates *= num_opts;
                    }
                }

                candidate_idx as f64 / num_candidates as f64
            }
            KeyCrackerPhase::FinishedSuccess => 1.,
            KeyCrackerPhase::FinishedFailure => 1.,
        }
    }

    pub fn do_work(&mut self) {
        match &mut self.phase {
            KeyCrackerPhase::SampleCollection { delay_timer } => {
                //Collect a sample and feed it to the predictor and test sample buffer
                let sample = (self.sample_provider)();
                self.key_predictor.accept_sample(&sample);
                self.test_sample_buf.accept_sample(&sample);

                //Occasionally check if we collected enough samples
                const READY_CHECK_PERIOD: usize = 2048;

                *delay_timer += 1;
                if *delay_timer >= READY_CHECK_PERIOD {
                    *delay_timer = 0;

                    if self.test_sample_buf.is_full()
                        && self.key_predictor.key_byte_infos().iter().all(|info| {
                            info.prediction_score() >= self.settings.key_predictor_threshold
                        })
                    {
                        //Move onto testing candidate keys
                        self.phase = KeyCrackerPhase::CandidateKeyTesting {
                            key_predictions: self
                                .key_predictor
                                .key_byte_infos()
                                .map(|info| info.prediction()),
                            strong_opt_idxs: [0; WepKey::LEN_104],
                        };
                    }
                }
            }
            KeyCrackerPhase::CandidateKeyTesting {
                key_predictions,
                strong_opt_idxs,
            } => {
                //Construct the current key and test it
                let mut key: [u8; WepKey::LEN_104] = [0; WepKey::LEN_104];
                let mut prev_sigma = 0u8;
                for i in 0..WepKey::LEN_104 {
                    //Get the sigma sum of the byte
                    let sigma = match key_predictions[i] {
                        KeyBytePrediction::Normal { sigma } => sigma,
                        KeyBytePrediction::Strong => {
                            let inv_rk = (1 + strong_opt_idxs[i]..i)
                                .map(|k| key[k] as isize + 3 + k as isize)
                                .sum::<isize>()
                                + 3
                                + i as isize;

                            (prev_sigma as isize - inv_rk).rem_euclid(256) as u8
                        }
                    };

                    //Calculate the key byte from the previous and this sigma
                    key[i] = (sigma as i32 - prev_sigma as i32).rem_euclid(256) as u8;
                    prev_sigma = sigma;
                }

                //Test the key
                if self.test_sample_buf.test_wep_key(&WepKey::Wep104Key(key)) {
                    //We found the key!
                    self.phase = KeyCrackerPhase::FinishedSuccess;
                    return;
                }

                //Move onto the next key
                let has_next_key = 'incr_loop: {
                    for i in (0..WepKey::LEN_104).rev() {
                        if key_predictions[i] != KeyBytePrediction::Strong {
                            continue;
                        }

                        //Increment the current option index
                        let num_opts = KeyBytePredictionInfo::num_strong_options(i);
                        strong_opt_idxs[i] += 1;

                        if strong_opt_idxs[i] >= num_opts {
                            //Move onto the next strong key byte
                            strong_opt_idxs[i] = 0;
                        } else {
                            break 'incr_loop true;
                        }
                    }
                    false
                };

                if !has_next_key {
                    //We went through all keys and didn't find one which matches :/
                    self.phase = KeyCrackerPhase::FinishedFailure;
                }
            }
            KeyCrackerPhase::FinishedSuccess | KeyCrackerPhase::FinishedFailure => {}
        }
    }
}

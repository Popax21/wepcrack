use std::sync::{atomic::AtomicBool, Arc};

use crate::{
    keycracker::{KeyBytePrediction, KeyPredictor, KeyTester, KeystreamSample, TestSampleBuffer},
    wep::WepKey,
};

#[derive(Debug, Clone, Copy)]
pub struct KeyCrackerSettings {
    //Sample collection settings
    pub key_predictor_normal_threshold: f64,
    pub key_predictor_strong_threshold: f64,

    //Test buffer settings
    pub num_test_samples: usize,
    pub test_sample_period: usize,
    pub test_sample_threshold: f64,
}

pub type KeyCrackerSampleProvider = dyn FnMut(&AtomicBool) -> Option<KeystreamSample> + Send + Sync;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum KeyCrackerPhase {
    SampleCollection,
    CandidateKeyTesting,
    FinishedSuccess,
    FinishedFailure,
}

pub(super) struct KeyCracker {
    phase: KeyCrackerPhase,
    delay_timer: usize,

    settings: KeyCrackerSettings,
    sample_provider: Box<KeyCrackerSampleProvider>,
    should_exit: Arc<AtomicBool>,

    key_predictor: KeyPredictor,
    test_sample_buf: TestSampleBuffer,
    key_tester: Option<KeyTester>,

    cracked_key: Option<WepKey>,
}

impl KeyCracker {
    pub fn new(
        settings: KeyCrackerSettings,
        sample_provider: Box<KeyCrackerSampleProvider>,
        should_exit: Arc<AtomicBool>,
    ) -> KeyCracker {
        KeyCracker {
            phase: KeyCrackerPhase::SampleCollection,
            delay_timer: 0,

            settings,
            sample_provider,
            should_exit,

            key_predictor: KeyPredictor::new(),
            test_sample_buf: TestSampleBuffer::new(
                settings.num_test_samples,
                settings.test_sample_period,
                settings.test_sample_threshold,
            ),
            key_tester: None,

            cracked_key: None,
        }
    }

    pub const fn settings(&self) -> &KeyCrackerSettings {
        &self.settings
    }

    pub const fn phase(&self) -> KeyCrackerPhase {
        self.phase
    }

    pub const fn is_running(&self) -> bool {
        !matches!(
            self.phase,
            KeyCrackerPhase::FinishedSuccess | KeyCrackerPhase::FinishedFailure
        )
    }

    pub const fn key_predictor(&self) -> &KeyPredictor {
        &self.key_predictor
    }

    pub const fn test_sample_buf(&self) -> &TestSampleBuffer {
        &self.test_sample_buf
    }

    pub const fn key_tester(&self) -> Option<&KeyTester> {
        self.key_tester.as_ref()
    }

    pub const fn cracked_key(&self) -> Option<&WepKey> {
        self.cracked_key.as_ref()
    }

    pub fn progress(&self) -> f64 {
        match self.phase {
            KeyCrackerPhase::SampleCollection => {
                //Aggregate progress of all key bytes towards the threshold
                self.key_predictor
                    .key_byte_infos()
                    .iter()
                    .map(|info| {
                        (info.prediction_score()
                            / (if matches!(
                                info.prediction(),
                                KeyBytePrediction::Normal { sigma: _ }
                            ) {
                                self.settings.key_predictor_normal_threshold
                            } else {
                                self.settings.key_predictor_strong_threshold
                            }))
                        .min(1.)
                    })
                    .sum::<f64>()
                    / self.key_predictor.key_byte_infos().len() as f64
            }
            KeyCrackerPhase::CandidateKeyTesting => {
                let tester = self.key_tester.as_ref().unwrap();
                tester.current_key_index() as f64 / tester.num_keys() as f64
            }
            KeyCrackerPhase::FinishedSuccess => 1.,
            KeyCrackerPhase::FinishedFailure => 1.,
        }
    }

    pub fn do_work(&mut self) {
        match self.phase {
            KeyCrackerPhase::SampleCollection => {
                //Collect a sample and feed it to the predictor and test sample buffer
                let Some(sample) = (self.sample_provider)(self.should_exit.as_ref()) else {
                    return;
                };
                self.key_predictor.accept_sample(&sample);
                self.test_sample_buf.accept_sample(&sample);

                //Occasionally check if we collected enough samples
                const READY_CHECK_PERIOD: usize = 2048;

                self.delay_timer += 1;
                if self.delay_timer >= READY_CHECK_PERIOD {
                    self.delay_timer = 0;

                    if self.test_sample_buf.is_full()
                        && self.key_predictor.key_byte_infos().iter().all(|info| {
                            info.prediction_score()
                                >= if matches!(
                                    info.prediction(),
                                    KeyBytePrediction::Normal { sigma: _ }
                                ) {
                                    self.settings.key_predictor_normal_threshold
                                } else {
                                    self.settings.key_predictor_strong_threshold
                                }
                        })
                    {
                        //Move onto testing candidate keys
                        self.phase = KeyCrackerPhase::CandidateKeyTesting;
                        self.key_tester = Some(KeyTester::new(
                            self.key_predictor
                                .key_byte_infos()
                                .map(|info| info.prediction()),
                        ));
                    }
                }
            }
            KeyCrackerPhase::CandidateKeyTesting => {
                let tester = self.key_tester.as_mut().unwrap();

                //Test a key
                if let Some(key) = tester.test_current_key(&self.test_sample_buf) {
                    //We found the key!
                    self.phase = KeyCrackerPhase::FinishedSuccess;
                    self.cracked_key = Some(key);
                    return;
                }

                //Move onto the next key
                if !tester.advance_to_next_key() {
                    //We went through all keys and didn't find one which matches :/
                    self.phase = KeyCrackerPhase::FinishedFailure;
                }
            }
            KeyCrackerPhase::FinishedSuccess | KeyCrackerPhase::FinishedFailure => {}
        }
    }
}

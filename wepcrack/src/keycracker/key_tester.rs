use crate::wep::WepKey;

use super::{KeyBytePrediction, TestSampleBuffer};

pub struct KeyTester {
    num_keys: usize,
    cur_key_idx: usize,

    key_predictions: [KeyBytePrediction; WepKey::LEN_104],
    strong_k_vals: [usize; WepKey::LEN_104],
}

impl KeyTester {
    pub fn new(key_predictions: [KeyBytePrediction; WepKey::LEN_104]) -> KeyTester {
        //Determine the total number of keys
        let mut num_keys = 1;
        for (idx, &pred) in key_predictions.iter().enumerate() {
            if pred == KeyBytePrediction::Strong {
                num_keys *= idx;
            }
        }
        assert!(num_keys >= 1);

        KeyTester {
            cur_key_idx: 0,
            num_keys,

            key_predictions,
            strong_k_vals: [1; WepKey::LEN_104],
        }
    }

    pub const fn num_keys(&self) -> usize {
        self.num_keys
    }

    pub const fn current_key_index(&self) -> usize {
        self.cur_key_idx
    }

    pub const fn is_at_end(&self) -> bool {
        self.cur_key_idx >= self.num_keys
    }

    pub fn current_key(&self) -> [u8; WepKey::LEN_104] {
        if self.is_at_end() {
            panic!("tried to get current key of an end-state KeyTester");
        }

        let mut key: [u8; WepKey::LEN_104] = [0; WepKey::LEN_104];
        let mut prev_sigma = 0u8;
        for i in 0..WepKey::LEN_104 {
            //Get the sigma sum of the byte
            let sigma = match self.key_predictions[i] {
                KeyBytePrediction::Normal { sigma } => sigma,
                KeyBytePrediction::Strong => {
                    let inv_rk = (self.strong_k_vals[i]..i)
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

        key
    }

    pub fn advance_to_next_key(&mut self) -> bool {
        if self.is_at_end() {
            return false;
        }

        for i in (0..WepKey::LEN_104).rev() {
            if self.key_predictions[i] != KeyBytePrediction::Strong {
                continue;
            }

            //Increment the k value for this strong byte
            self.strong_k_vals[i] += 1;
            if self.strong_k_vals[i] > i {
                //Reset k and move onto the next strong key byte
                self.strong_k_vals[i] = 1;
            } else {
                self.cur_key_idx += 1;
                return true;
            }
        }
        panic!("unable to advance to next key");
    }

    pub fn test_current_key(&self, test_sample_buf: &TestSampleBuffer) -> Option<WepKey> {
        let key = WepKey::Wep104Key(self.current_key());
        if test_sample_buf.test_wep_key(&key) {
            Some(key)
        } else {
            None
        }
    }
}

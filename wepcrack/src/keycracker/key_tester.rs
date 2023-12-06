use crate::wep::WepKey;

use super::{KeyBytePrediction, TestSampleBuffer};

pub struct KeyTester {
    num_keys: usize,
    cur_key_idx: usize,
    cur_l_idxs: [usize; WepKey::LEN_104],

    key_predictions: [KeyBytePrediction; WepKey::LEN_104],
    maybe_wep40: bool,
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

        //Check if the key could be a WEP-40 key
        let maybe_wep40 = key_predictions[WepKey::LEN_40..]
            .iter()
            .all(|&pred| pred == KeyBytePrediction::Strong);

        KeyTester {
            cur_key_idx: 0,
            num_keys,
            cur_l_idxs: key_predictions.map(|pred| match pred {
                KeyBytePrediction::Strong => 1,
                _ => usize::MAX,
            }),

            key_predictions,
            maybe_wep40,
        }
    }

    pub const fn key_predictions(&self) -> [KeyBytePrediction; WepKey::LEN_104] {
        self.key_predictions
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

    pub const fn is_maybe_wep40(&self) -> bool {
        self.maybe_wep40
    }

    pub const fn current_l_indices(&self) -> [usize; WepKey::LEN_104] {
        self.cur_l_idxs
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
                    let inv_rk = (self.cur_l_idxs[i]..i)
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

        for i in 0..WepKey::LEN_104 {
            if self.key_predictions[i] != KeyBytePrediction::Strong {
                continue;
            }

            //If we ever advance to a non-WEP40 byte then our key isn't one
            if i >= WepKey::LEN_40 {
                self.maybe_wep40 = false;
            }

            //Increment the k value for this strong byte
            self.cur_l_idxs[i] += 1;
            if self.cur_l_idxs[i] > i {
                //Reset k and move onto the next strong key byte
                self.cur_l_idxs[i] = 1;
            } else {
                self.cur_key_idx += 1;
                return true;
            }
        }
        panic!("unable to advance to next key");
    }

    pub fn test_current_key(&self, test_sample_buf: &TestSampleBuffer) -> Option<WepKey> {
        let key = self.current_key();

        //Test as a WEP-104 key
        let key104 = WepKey::Wep104Key(key);
        if test_sample_buf.test_wep_key(&key104) {
            return Some(key104);
        }

        //Test as a WEP-40 key if it might be one
        if self.maybe_wep40 {
            let mut key40 = [0u8; WepKey::LEN_40];
            key40.copy_from_slice(&key[..WepKey::LEN_40]);

            let key40 = WepKey::Wep40Key(key40);
            if test_sample_buf.test_wep_key(&key40) {
                return Some(key40);
            }
        }

        None
    }
}

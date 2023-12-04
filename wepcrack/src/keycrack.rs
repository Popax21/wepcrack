use std::collections::VecDeque;

use crate::{
    rc4::RC4Cipher,
    wep::{WepIV, WepKey},
};

//Implementation of "Breaking 104 bit WEP in less than 60 seconds" (https://eprint.iacr.org/2007/120.pdf)

#[derive(Clone, Copy, Default)]
pub struct WepKeystreamSample {
    pub keystream: [u8; WepKeystreamSample::KEYSTREAM_LEN],
    pub iv: WepIV,
}

impl WepKeystreamSample {
    pub const KEYSTREAM_LEN: usize = 16;
}

pub type WepKeystreamSampleProvider = dyn (Fn() -> WepKeystreamSample) + Send + Sync;

pub struct WepKeyCrackerKeyByteInfo {
    pub candidate_sigma: u8,

    pub p_candidate: f64,
    pub p_correct: f64,
    pub p_equal: f64,

    pub err_strong: f64,
    pub err_normal: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct WepKeyCrackerSettings {
    pub num_test_samples: usize,
    pub test_sample_fract: f64,
    pub test_sample_period: usize,
}

pub struct WepKeyCracker {
    settings: WepKeyCrackerSettings,

    p_correct: [f64; WepKey::LEN_104],

    num_samples: usize,
    sigma_votes: [[usize; 256]; WepKey::LEN_104],

    test_samples: VecDeque<WepKeystreamSample>,
    test_sample_counter: usize,
}

impl WepKeyCracker {
    pub fn new(settings: &WepKeyCrackerSettings) -> WepKeyCracker {
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

        WepKeyCracker {
            settings: *settings,

            p_correct,

            num_samples: 0,
            sigma_votes: [[0; 256]; WepKey::LEN_104],

            test_samples: VecDeque::with_capacity(settings.num_test_samples),
            test_sample_counter: 0,
        }
    }

    pub const fn num_samples(&self) -> usize {
        self.num_samples
    }

    pub fn accept_sample(&mut self, sample: &WepKeystreamSample) {
        //Do a partial keyschedule to determine S_3 and j_3
        let (s_3, j_3) = {
            let mut rc4 = RC4Cipher::default();
            rc4.do_partial_keyschedule(&sample.iv);
            (rc4.s, rc4.j)
        };

        //Determine the inverse permutation of S_3
        let mut sinv_3 = [0u8; 256];
        for i in 0..256 {
            sinv_3[s_3[i] as usize] = i as u8;
        }

        //Calculate approximate sigma sums for all key bytes
        let mut s3_sum: usize = 0;
        for i in 0..WepKey::LEN_104 {
            //Update the sum of S3 in the range of 3 to 3+i
            s3_sum += s_3[3 + i] as usize;

            //Calculate sigma
            let sigma = sinv_3
                [(3 + i as isize - sample.keystream[2 + i] as isize).rem_euclid(256) as usize]
                as isize
                - (j_3 + s3_sum) as isize;

            //Add a vote for this sigma
            self.sigma_votes[i][sigma.rem_euclid(256) as usize] += 1;
        }

        //Increment the sample counter
        self.num_samples += 1;

        //Enqueue every n-th sample into the test sample queue
        self.test_sample_counter += 1;
        if self.test_sample_counter >= self.settings.test_sample_period {
            //Don't keep more than a max number of samples around
            while self.test_samples.len() > self.settings.num_test_samples {
                self.test_samples.pop_front();
            }

            self.test_samples.push_back(*sample);
        }
    }

    pub fn calc_key_byte_info(&self, key_idx: usize) -> WepKeyCrackerKeyByteInfo {
        //Find the index of the candidate sigma (= the one with the most votes)
        let candidate_sigma = self.sigma_votes[key_idx]
            .iter()
            .enumerate()
            .max_by(|(_, v1), (_, v2)| v1.cmp(v2))
            .unwrap()
            .0;

        //Calculate err_strong and err_weak
        let p_equal = 1f64 / 256f64;
        let p_correct = self.p_correct[key_idx];
        let p_wrong = (1f64 - p_correct) / 255f64;

        let mut err_strong = 0f64;
        let mut err_normal = 0f64;
        for (sigma, &votes) in self.sigma_votes[key_idx].iter().enumerate() {
            let frac = votes as f64 / self.num_samples as f64;

            err_strong += (frac - p_equal) * (frac - p_equal);

            if sigma == candidate_sigma {
                err_normal += (frac - p_correct) * (frac - p_correct);
            } else {
                err_normal += (frac - p_wrong) * (frac - p_wrong);
            }
        }

        WepKeyCrackerKeyByteInfo {
            candidate_sigma: candidate_sigma as u8,

            p_candidate: self.sigma_votes[key_idx][candidate_sigma] as f64
                / self.num_samples as f64,
            p_correct,
            p_equal,

            err_strong,
            err_normal,
        }
    }

    fn test_wep_key(&self, key: &WepKey) -> bool {
        //Calculate the maximum number of incorrect samples (the negative threshold)
        let threshold =
            (self.test_samples.len() as f64 * self.settings.test_sample_fract).ceil() as usize;
        let neg_threshold = self.test_samples.len() - threshold;

        //Check samples
        let mut neg_samples = 0;
        for sample in &self.test_samples {
            //Compute the keystream based on the sample IV
            let mut rc4 = key.create_rc4(&sample.iv);

            let mut keystream = [0u8; WepKeystreamSample::KEYSTREAM_LEN];
            rc4.gen_keystream(&mut keystream);

            //Compare with the correct sample keystream
            if keystream.eq(&sample.keystream) {
                continue;
            }

            //Check if the negative sample threshold has been crossed
            neg_samples += 1;
            if neg_samples >= neg_threshold {
                return false;
            }
        }

        true
    }
}

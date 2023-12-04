use crate::{rc4::RC4Cipher, wep::WepKey};

use super::{KeyByteInfo, KeystreamSample, TestSampleBuffer};

#[derive(Debug, Clone, Copy)]
pub struct KeyCrackerSettings {
    pub num_test_samples: usize,
    pub test_sample_period: usize,
    pub test_sample_threshold: f64,
}

pub struct WepKeyCracker {
    num_samples: usize,
    sigma_votes: [[usize; 256]; WepKey::LEN_104],

    test_sample_buf: TestSampleBuffer,
}

impl WepKeyCracker {
    pub fn new(settings: &KeyCrackerSettings) -> WepKeyCracker {
        WepKeyCracker {
            num_samples: 0,
            sigma_votes: [[0; 256]; WepKey::LEN_104],

            test_sample_buf: TestSampleBuffer::new(
                settings.num_test_samples,
                settings.test_sample_period,
                settings.test_sample_threshold,
            ),
        }
    }

    pub const fn num_samples(&self) -> usize {
        self.num_samples
    }

    pub fn accept_sample(&mut self, sample: &KeystreamSample) {
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

        //Add the sample to the test sample buffer
        self.test_sample_buf.accept_sample(sample);
    }

    pub fn calc_key_byte_info(&self, key_idx: usize) -> KeyByteInfo {
        KeyByteInfo::from_sigma_votes(key_idx, &self.sigma_votes[key_idx], self.num_samples)
    }
}

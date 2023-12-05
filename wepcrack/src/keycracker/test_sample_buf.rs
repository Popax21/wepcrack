use std::collections::VecDeque;

use crate::wep::WepKey;

use super::KeystreamSample;

pub(super) struct TestSampleBuffer {
    samples: VecDeque<KeystreamSample>,
    buffer_size: usize,

    period_timer: usize,
    sample_period: usize,

    test_threshold_fract: f64,
}

impl TestSampleBuffer {
    pub fn new(
        buffer_size: usize,
        sample_period: usize,
        test_threshold_fract: f64,
    ) -> TestSampleBuffer {
        TestSampleBuffer {
            samples: VecDeque::with_capacity(buffer_size),
            buffer_size,

            period_timer: 0,
            sample_period,

            test_threshold_fract,
        }
    }

    pub fn num_samples(&self) -> usize {
        self.samples.len()
    }

    pub fn is_ready(&self) -> bool {
        self.samples.len() >= self.buffer_size
    }

    pub fn accept_sample(&mut self, sample: &KeystreamSample) {
        //Only accept every n-th sample
        self.period_timer += 1;
        if self.period_timer < self.sample_period {
            return;
        }

        self.period_timer = 0;

        //Remove old samples from the buffer
        while self.samples.len() >= self.buffer_size {
            self.samples.pop_front();
        }

        //Add the sample to the buffer
        self.samples.push_back(*sample);
    }

    pub fn test_wep_key(&self, key: &WepKey) -> bool {
        //Calculate the maximum number of incorrect samples (the negative threshold)
        let threshold = (self.samples.len() as f64 * self.test_threshold_fract).ceil() as usize;
        let neg_threshold = self.samples.len() - threshold;

        //Check samples
        let mut neg_samples = 0;
        for sample in &self.samples {
            //Compute the keystream based on the sample IV
            let mut rc4 = key.create_rc4(&sample.iv);

            let mut keystream = [0u8; KeystreamSample::KEYSTREAM_LEN];
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

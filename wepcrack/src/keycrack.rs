use crate::rc4::RC4Cipher;

//Implementation of "Breaking 104 bit WEP in less than 60 seconds" (https://eprint.iacr.org/2007/120.pdf)

pub const WEP_KEY_SIZE: usize = 13; //104-bit key
pub type WepIV = [u8; 3];
pub type WepKey = [u8; WEP_KEY_SIZE];

pub type WepKeystreamSample = [u8; 2 + WEP_KEY_SIZE];

pub struct WepKeyCracker {
    num_samples: usize,
    sigma_votes: [[usize; 256]; WEP_KEY_SIZE],
}

impl WepKeyCracker {
    pub const fn num_samples(&self) -> usize {
        self.num_samples
    }

    pub fn accept_sample(&mut self, iv: &WepIV, keystream: &WepKeystreamSample) {
        //Do a partial keyschedule to determine S_3 and j_3
        let (s_3, j_3) = {
            let mut rc4 = RC4Cipher::default();
            rc4.do_partial_keyschedule(iv, 3);
            (rc4.s, rc4.j)
        };

        //Determine the inverse permutation of S_3
        let mut sinv_3 = [0u8; 256];
        for i in 0..256 {
            sinv_3[s_3[i] as usize] = i as u8;
        }

        //Calculate approximate sigma sums for all key bytes
        let mut s3_sum: usize = 0;
        for i in 0..WEP_KEY_SIZE {
            //Update the sum of S3 in the range of 3 to 3+i
            s3_sum += s_3[3 + i] as usize;

            //Calculate sigma
            let sigma =
                sinv_3[3 + i + keystream[2 + i] as usize] as isize - (j_3 + s3_sum) as isize;

            //Add a vote for this sigma
            self.sigma_votes[i][sigma.rem_euclid(256) as usize] += 1;
        }

        //Increment the sample counter
        self.num_samples += 1;
    }
}

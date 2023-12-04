pub struct RC4Cipher {
    pub s: [u8; 256],
    pub i: usize,
    pub j: usize,
}

impl Default for RC4Cipher {
    fn default() -> Self {
        //Init the permutation to the identity permutation
        let mut s = [0u8; 256];
        for (i, sb) in s.iter_mut().enumerate() {
            *sb = i as u8;
        }

        Self { s, i: 0, j: 0 }
    }
}

impl RC4Cipher {
    pub fn from_key(key: &[u8]) -> RC4Cipher {
        let mut cipher = RC4Cipher::default();

        //Do key scheduling
        let mut full_key = [0u8; 256];
        let mut off = 0;
        while off < 256 {
            let len = key.len().min(256 - off);
            full_key[off..off + len].copy_from_slice(&key[..len]);
            off += len;
        }
        cipher.do_partial_keyschedule(&full_key);

        //Reset i, j
        cipher.i = 0;
        cipher.j = 0;

        cipher
    }

    pub fn do_partial_keyschedule(&mut self, key: &[u8]) {
        assert!(self.i + key.len() <= 256);
        for &kb in key {
            //Update j
            self.j = (self.j + self.s[self.i] as usize + kb as usize) % 256;

            //Swap permutation elements
            (self.s[self.i], self.s[self.j]) = (self.s[self.j], self.s[self.i]);

            //Increment i
            self.i += 1;
        }
    }

    pub fn gen_keystream_byte(&mut self) -> u8 {
        //Update i, j
        self.i = (self.i + 1) % 256;
        self.j = (self.j + self.s[self.i] as usize) % 256;

        //Swap permutation elements
        (self.s[self.i], self.s[self.j]) = (self.s[self.j], self.s[self.i]);

        //Lookup keystream byte
        self.s[(self.s[self.j] as usize + self.s[self.i] as usize) % 256]
    }

    pub fn gen_keystream(&mut self, keystream: &mut [u8]) {
        for ksb in keystream.iter_mut() {
            *ksb = self.gen_keystream_byte();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rc4() {
        //Test multiple keystream test vectors
        for (key, hex_keystream) in [
            ("Key", "EB9F7781B734CA72A719"),
            ("Secret", "04D46B053CA87B59"),
        ] {
            let mut gen = RC4Cipher::from_key(key.as_bytes());
            for i in (0..hex_keystream.len()).step_by(2) {
                assert_eq!(
                    gen.gen_keystream_byte(),
                    u8::from_str_radix(&hex_keystream[i..i + 2], 16).unwrap()
                );
            }
        }
    }
}

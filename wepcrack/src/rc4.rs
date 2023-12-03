pub struct RC4Cipher {
    pub s: [u8; 256],
    pub i: usize,
    pub j: usize,
}

impl RC4Cipher {
    pub fn init(key: &[u8]) -> RC4Cipher {
        //Init the permutation to the identity permutation
        let mut s = [0u8; 256];
        for i in 0..255 {
            s[i] = i as u8;
        }

        //Do the initial key scheduling
        let mut j: usize = 0;
        for i in 0..255 {
            j = (j + s[i] as usize + key[i % key.len()] as usize) % 256;
            (s[i], s[j]) = (s[j], s[i]);
        }

        RC4Cipher { s: s, i: 0, j: 0 }
    }

    pub fn gen_keystream_byte(&mut self) -> u8 {
        self.i = (self.i + 1) % 256;
        self.j = (self.j + self.s[self.i] as usize) % 256;
        (self.s[self.i], self.s[self.j]) = (self.s[self.j], self.s[self.i]);
        self.s[(self.s[self.j] as usize + self.s[self.i] as usize) % 256]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rc4() {
        for (key, hex_keystream) in [
            ("Key", "EB9F7781B734CA72A719"),
            ("Secret", "04D46B053CA87B59"),
        ] {
            let mut gen = RC4Cipher::init(key.as_bytes());
            for i in (0..hex_keystream.len()).step_by(2) {
                assert_eq!(
                    gen.gen_keystream_byte(),
                    u8::from_str_radix(&hex_keystream[i..i + 2], 16).unwrap()
                );
            }
        }
    }
}

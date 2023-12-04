use crate::rc4::RC4Cipher;

pub type WepIV = [u8; 3];

pub enum WepKey {
    Wep40Key([u8; 5]),
    Wep104Key([u8; 13]),
}

impl WepKey {
    pub const LEN_40: usize = 5;
    pub const LEN_104: usize = 13;

    pub fn create_rc4(&self, iv: &WepIV) -> RC4Cipher {
        match self {
            Self::Wep40Key(wep_key) => {
                let mut rc4_key = [0u8; 8];
                rc4_key[..3].copy_from_slice(iv);
                rc4_key[3..].copy_from_slice(wep_key);
                RC4Cipher::from_key(&rc4_key)
            }
            Self::Wep104Key(wep_key) => {
                let mut rc4_key = [0u8; 16];
                rc4_key[..3].copy_from_slice(iv);
                rc4_key[3..].copy_from_slice(wep_key);
                RC4Cipher::from_key(&rc4_key)
            }
        }
    }
}

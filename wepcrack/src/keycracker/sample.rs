use crate::wep::WepIV;

#[derive(Clone, Copy, Default)]
pub struct KeystreamSample {
    pub keystream: [u8; KeystreamSample::KEYSTREAM_LEN],
    pub iv: WepIV,
}

impl KeystreamSample {
    pub const KEYSTREAM_LEN: usize = 16;
}

pub type KeystreamSampleProvider = dyn (Fn() -> KeystreamSample) + Send + Sync;

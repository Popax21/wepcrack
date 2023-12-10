use num_enum::TryFromPrimitive;

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, TryFromPrimitive)]
pub enum NL80211ChannelWidth {
    Mhz20NoHT,
    Mhz20,
    Mhz40,
    Mhz80,
    Mhz80P80,
    Mhz160,
    Mhz5,
    Mhz10,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NL80211Channel {
    pub freq_mhz: u32,
    pub width: NL80211ChannelWidth,
    pub center_freq1_mhz: u32,
    pub center_freq2_mhz: u32,
}

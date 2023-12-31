use std::{
    fmt::Display,
    ops::{RangeInclusive, Rem},
};

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

impl NL80211ChannelWidth {
    pub fn bandwidth(self) -> u32 {
        match self {
            Self::Mhz20NoHT => 20,
            Self::Mhz20 => 20,
            Self::Mhz40 => 40,
            Self::Mhz80 => 80,
            Self::Mhz80P80 => 160,
            Self::Mhz160 => 160,
            Self::Mhz5 => 5,
            Self::Mhz10 => 10,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NL80211Channel {
    Channel20NoHT { channel: u32 },
    ChannelHT20 { channel: u32 },
    ChannelHT40 { main_channel: u32, aux_channel: u32 },
    ChannelVHT80 { main_channel: u32, aux_channel: u32 },
    ChannelVHT160 { main_channel: u32, aux_channel: u32 },
}

impl NL80211Channel {
    pub fn all_channels() -> Box<dyn Iterator<Item = NL80211Channel>> {
        let iter = std::iter::empty();

        macro_rules! chain_iter {
            ($iter:ident, $($new_iter:expr),+) => {
                $( let $iter = $iter.chain($new_iter); )+
            };
        }

        //2.4GHz channels
        chain_iter!(
            iter,
            (1..=14).map(|channel| Self::mhz20_channel(channel).unwrap()),
            (1..=14).map(|channel| Self::ht20_channel(channel).unwrap()),
            (1..=(13 - 4)).map(|channel| Self::ht40_channel(channel, channel + 4).unwrap()), //HT40+
            ((1 + 4)..=13).map(|channel| Self::ht40_channel(channel, channel - 4).unwrap())  //HT40-
        );

        //5GHz channels
        chain_iter!(
            iter,
            (32..=144)
                .step_by(4)
                .map(|channel| Self::mhz20_channel(channel).unwrap()),
            (32..=144)
                .step_by(4)
                .map(|channel| Self::ht20_channel(channel).unwrap()),
            //HT40+
            (36..=(144 - 4))
                .step_by(8)
                .map(|channel| Self::ht40_channel(channel, channel + 4).unwrap()),
            //HT40-
            ((36 + 4)..=144)
                .step_by(8)
                .map(|channel| Self::ht40_channel(channel, channel - 4).unwrap()),
            //VHT80
            (38..=(142 - 8))
                .step_by(16)
                .map(|channel| Self::vht80_channel(channel, channel + 8).unwrap()),
            //VHT160
            (42..=(138 - 16))
                .step_by(32)
                .map(|channel| Self::vht160_channel(channel, channel + 16).unwrap())
        );

        Box::new(iter)
    }

    pub fn new(
        freq: u32,
        width: NL80211ChannelWidth,
        center_freq1: Option<u32>,
        _center_freq2: Option<u32>,
    ) -> Option<NL80211Channel> {
        let Some(channel) = Self::freq_to_channel_idx(freq) else {
            return None;
        };

        match width {
            NL80211ChannelWidth::Mhz20NoHT => Self::mhz20_channel(channel),
            NL80211ChannelWidth::Mhz20 => Self::ht20_channel(channel),
            NL80211ChannelWidth::Mhz40 => {
                let Some(center_freq1) = center_freq1 else {
                    return None;
                };
                let Some(center_freq1) = Self::freq_to_channel_idx(center_freq1) else {
                    return None;
                };
                if center_freq1.abs_diff(channel) != 2 {
                    return None;
                }

                Self::ht40_channel(center_freq1, 2 * channel - center_freq1)
            }
            NL80211ChannelWidth::Mhz80 => {
                let Some(center_freq1) = center_freq1 else {
                    return None;
                };
                let Some(center_freq1) = Self::freq_to_channel_idx(center_freq1) else {
                    return None;
                };
                if center_freq1.abs_diff(channel) != 4 {
                    return None;
                }

                Self::ht40_channel(center_freq1, 2 * channel - center_freq1)
            }
            NL80211ChannelWidth::Mhz160 => {
                let Some(center_freq1) = center_freq1 else {
                    return None;
                };
                let Some(center_freq1) = Self::freq_to_channel_idx(center_freq1) else {
                    return None;
                };
                if center_freq1.abs_diff(channel) != 8 {
                    return None;
                }

                Self::ht40_channel(center_freq1, 2 * channel - center_freq1)
            }
            _ => None,
        }
    }

    pub fn mhz20_channel(channel: u32) -> Option<NL80211Channel> {
        Self::channel_idx_to_band(channel).map(|_| NL80211Channel::Channel20NoHT { channel })
    }

    pub fn ht20_channel(channel: u32) -> Option<NL80211Channel> {
        Self::channel_idx_to_band(channel).map(|_| NL80211Channel::ChannelHT20 { channel })
    }

    pub fn ht40_channel(main_channel: u32, aux_channel: u32) -> Option<NL80211Channel> {
        if main_channel.abs_diff(aux_channel) != 4 {
            return None;
        }

        Self::channel_idx_to_band(main_channel)
            .zip(Self::channel_idx_to_band(aux_channel))
            .and_then(|(main_band, aux_band)| {
                if main_band == aux_band {
                    if main_band == NL80211ChannelBand::Band5Ghz
                        && (main_channel.rem(4) != 0 || aux_channel.rem(4) != 0)
                    {
                        return None;
                    }

                    Some(NL80211Channel::ChannelHT40 {
                        main_channel,
                        aux_channel,
                    })
                } else {
                    None
                }
            })
    }

    pub fn vht80_channel(main_channel: u32, aux_channel: u32) -> Option<NL80211Channel> {
        if main_channel.abs_diff(aux_channel) != 8
            || Self::channel_idx_to_band(main_channel) != Some(NL80211ChannelBand::Band5Ghz)
            || Self::channel_idx_to_band(aux_channel) != Some(NL80211ChannelBand::Band5Ghz)
        {
            return None;
        }

        if (main_channel - 2).rem(8) != 4 || (aux_channel - 2).rem(8) != 4 {
            return None;
        }

        Some(NL80211Channel::ChannelVHT80 {
            main_channel,
            aux_channel,
        })
    }

    pub fn vht160_channel(main_channel: u32, aux_channel: u32) -> Option<NL80211Channel> {
        if main_channel.abs_diff(aux_channel) != 16
            || Self::channel_idx_to_band(main_channel) != Some(NL80211ChannelBand::Band5Ghz)
            || Self::channel_idx_to_band(aux_channel) != Some(NL80211ChannelBand::Band5Ghz)
        {
            return None;
        }

        if main_channel.rem(16) != 10 || aux_channel.rem(16) != 10 {
            return None;
        }

        Some(NL80211Channel::ChannelVHT160 {
            main_channel,
            aux_channel,
        })
    }

    //There are a whole lot more bands + associated channels
    //But we only really care about those in the 2.4GHz and 5.0GHhz bands
    pub fn is_valid_20mhz_channel_idx(idx: u32) -> bool {
        match idx {
            //Channels 1-14: 2.4GHz
            1..=14 => true,

            //Channel 32-144: 5.160Ghz
            32..=144 => idx.rem(4) == 0,

            _ => false,
        }
    }

    pub fn channel_idx_to_band(idx: u32) -> Option<NL80211ChannelBand> {
        match idx {
            //Channels 1-14: 2.4GHz
            1..=14 => Some(NL80211ChannelBand::Band2400Mhz),

            //Channel 32-144: 5.160Ghz
            32..=144 => Some(NL80211ChannelBand::Band5Ghz),

            _ => None,
        }
    }

    pub fn channel_idx_to_freq(idx: u32) -> Option<u32> {
        match idx {
            //Channels 1-13: 2.412GHz 5MHz spacing
            1..=13 => Some(2412 + 5 * (idx - 1)),

            //Channel 14: 2.484Ghz
            14 => Some(2484),

            //Channel 32-144: 5.160Ghz 5MHz spacing
            32..=144 => Some(5160 + 5 * (idx - 32)),

            _ => None,
        }
    }

    pub fn freq_to_channel_idx(freq: u32) -> Option<u32> {
        match freq {
            //Channels 1-13: 2.412GHz 5MHz spacing
            2412..=2472 => {
                if (freq - 2412).rem(5) == 0 {
                    Some(1 + (freq - 2412) / 5)
                } else {
                    None
                }
            }

            //Channel 14: 2.484Ghz
            2484 => Some(14),

            //Channel 32-144: 5.160Ghz 5MHz spacing
            32..=144 => {
                if (freq - 5885).rem(5) == 0 {
                    Some(32 + (freq - 5885) / 5)
                } else {
                    None
                }
            }

            _ => None,
        }
    }

    pub fn band(&self) -> NL80211ChannelBand {
        match self {
            NL80211Channel::Channel20NoHT { channel } | NL80211Channel::ChannelHT20 { channel } => {
                Self::channel_idx_to_band(*channel).unwrap()
            }
            NL80211Channel::ChannelHT40 {
                main_channel,
                aux_channel: _,
            }
            | NL80211Channel::ChannelVHT80 {
                main_channel,
                aux_channel: _,
            }
            | NL80211Channel::ChannelVHT160 {
                main_channel,
                aux_channel: _,
            } => Self::channel_idx_to_band(*main_channel).unwrap(),
        }
    }

    pub fn frequency(&self) -> u32 {
        match self {
            NL80211Channel::Channel20NoHT { channel } | NL80211Channel::ChannelHT20 { channel } => {
                Self::channel_idx_to_freq(*channel).unwrap()
            }
            NL80211Channel::ChannelHT40 {
                main_channel,
                aux_channel,
            }
            | NL80211Channel::ChannelVHT80 {
                main_channel,
                aux_channel,
            }
            | NL80211Channel::ChannelVHT160 {
                main_channel,
                aux_channel,
            } => Self::channel_idx_to_freq((*main_channel + *aux_channel) / 2).unwrap(),
        }
    }

    pub fn width(&self) -> NL80211ChannelWidth {
        match self {
            NL80211Channel::Channel20NoHT { channel: _ } => NL80211ChannelWidth::Mhz20NoHT,
            NL80211Channel::ChannelHT20 { channel: _ } => NL80211ChannelWidth::Mhz20,
            NL80211Channel::ChannelHT40 {
                main_channel: _,
                aux_channel: _,
            } => NL80211ChannelWidth::Mhz40,
            NL80211Channel::ChannelVHT80 {
                main_channel: _,
                aux_channel: _,
            } => NL80211ChannelWidth::Mhz80,
            NL80211Channel::ChannelVHT160 {
                main_channel: _,
                aux_channel: _,
            } => NL80211ChannelWidth::Mhz160,
        }
    }

    pub fn freq_range(&self) -> RangeInclusive<u32> {
        let center_freq = self.frequency();
        let bandwidth = self.width().bandwidth();
        (center_freq - bandwidth / 2)..=(center_freq + bandwidth / 2)
    }

    pub fn channel_range(&self) -> RangeInclusive<u32> {
        match self {
            NL80211Channel::Channel20NoHT { channel } | NL80211Channel::ChannelHT20 { channel } => {
                *channel..=*channel
            }
            NL80211Channel::ChannelHT40 {
                main_channel,
                aux_channel,
            } => *main_channel.min(aux_channel)..=*main_channel.max(aux_channel),
            NL80211Channel::ChannelVHT80 {
                main_channel,
                aux_channel,
            } => (*main_channel.min(aux_channel) - 2)..=(*main_channel.max(aux_channel) + 2),
            NL80211Channel::ChannelVHT160 {
                main_channel,
                aux_channel,
            } => {
                (*main_channel.min(aux_channel) - 2 - 4)..=(*main_channel.max(aux_channel) + 2 + 4)
            }
        }
    }

    pub fn nla_frequency(&self) -> u32 {
        match self {
            NL80211Channel::Channel20NoHT { channel } | NL80211Channel::ChannelHT20 { channel } => {
                Self::channel_idx_to_freq(*channel).unwrap()
            }
            NL80211Channel::ChannelHT40 {
                main_channel,
                aux_channel: _,
            } => Self::channel_idx_to_freq(*main_channel).unwrap(),
            NL80211Channel::ChannelVHT80 {
                main_channel,
                aux_channel: _,
            } => Self::channel_idx_to_freq(*main_channel - 2).unwrap(),
            NL80211Channel::ChannelVHT160 {
                main_channel,
                aux_channel: _,
            } => Self::channel_idx_to_freq(*main_channel - 2 - 4).unwrap(),
        }
    }
}

impl Display for NL80211Channel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NL80211Channel::Channel20NoHT { channel } => write!(
                f,
                "  {channel:>3}    | {freq:5.3}Ghz @ 20MHz",
                freq = self.frequency() as f64 / 1000.
            ),
            NL80211Channel::ChannelHT20 { channel } => write!(
                f,
                "  {channel:>3}    | {freq:5.3}Ghz @ 20MHz (HT20)",
                freq = self.frequency() as f64 / 1000.
            ),
            NL80211Channel::ChannelHT40 {
                main_channel,
                aux_channel,
            } => match self.band() {
                NL80211ChannelBand::Band2400Mhz => {
                    write!(
                        f,
                        " {main_channel:>3}-{aux_channel:<3} | {freq:5.3}Ghz @ 40Mhz (HT40{sign})",
                        freq = self.frequency() as f64 / 1000.,
                        sign = if main_channel < aux_channel { "+" } else { "-" }
                    )
                }
                NL80211ChannelBand::Band5Ghz => {
                    write!(
                        f,
                        "{channel:>3}[{main_channel:>3}] | {freq:5.3}Ghz @ 40Mhz (HT40{sign})",
                        channel = (main_channel + aux_channel) / 2,
                        freq = self.frequency() as f64 / 1000.,
                        sign = if main_channel < aux_channel { "+" } else { "-" }
                    )
                }
            },
            NL80211Channel::ChannelVHT80 {
                main_channel,
                aux_channel,
            } => write!(
                f,
                "{channel:>3}[{main_channel:>3}] | {freq:5.3}Ghz @ 80Mhz (VHT80)",
                channel = (main_channel + aux_channel) / 2,
                freq = self.frequency() as f64 / 1000.
            ),
            NL80211Channel::ChannelVHT160 {
                main_channel,
                aux_channel,
            } => write!(
                f,
                "{channel:>3}[{main_channel:>3}] | {freq:5.3}Ghz @ 160Mhz (VHT160)",
                channel = (main_channel + aux_channel) / 2,
                freq = self.frequency() as f64 / 1000.
            ),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum NL80211ChannelBand {
    Band2400Mhz,
    Band5Ghz,
}

impl NL80211ChannelBand {
    pub fn band_from_freq(freq: u32) -> Option<NL80211ChannelBand> {
        match freq {
            2401..=2495 => Some(NL80211ChannelBand::Band2400Mhz),
            5150..=5730 => Some(NL80211ChannelBand::Band5Ghz),
            _ => None,
        }
    }
}

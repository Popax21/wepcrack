use netlink_packet_utils::{
    nla::{DefaultNla, Nla, NlaBuffer, NlasIterator},
    DecodeError, Emitable, Parseable,
};
use num_enum::TryFromPrimitive;

use crate::{
    nl80211::{NL80211Attribute, NL80211AttributeTag, NL80211Command},
    steal_msg_attr,
};

use super::{
    attr_macro::{attr_size, attr_tag, emit_attr, parse_attr},
    NL80211Connection, NL80211Message, NL80211Wiphy,
};

#[derive(Debug, Clone)]
pub struct NL80211RegulatoryDomain {
    country_code: String,
    dfs_region: Option<u8>,
    rules: Vec<NL80211RegulatoryRule>,
}

impl NL80211RegulatoryDomain {
    fn from_message(mut msg: NL80211Message) -> NL80211RegulatoryDomain {
        msg.verify_cmd(NL80211Command::GetReg);

        steal_msg_attr!(RegAlpha2(country_code) = msg);
        steal_msg_attr!(RegRules(rules) = msg);

        let dfs_region = {
            if let Some(NL80211Attribute::DFSRegion(dfs_region)) =
                msg.steal_attribute(NL80211AttributeTag::DFSRegion)
            {
                Some(dfs_region)
            } else {
                None
            }
        };

        NL80211RegulatoryDomain {
            country_code,
            dfs_region,
            rules,
        }
    }

    pub fn query_global(
        nl80211_con: &NL80211Connection,
    ) -> anyhow::Result<NL80211RegulatoryDomain> {
        Ok(Self::from_message(nl80211_con.send_get_request(
            NL80211Message {
                cmd: NL80211Command::GetReg,
                nlas: vec![],
            },
        )?))
    }

    pub fn query_for_wiphy(
        nl80211_con: &NL80211Connection,
        wiphy: &NL80211Wiphy,
    ) -> anyhow::Result<NL80211RegulatoryDomain> {
        Ok(Self::from_message(nl80211_con.send_get_request(
            NL80211Message {
                cmd: NL80211Command::GetReg,
                nlas: vec![NL80211Attribute::WiphyIndex(wiphy.index())],
            },
        )?))
    }

    pub fn country_code(&self) -> &str {
        &self.country_code
    }

    pub const fn dfs_region(&self) -> Option<u8> {
        self.dfs_region
    }

    pub fn rules(&self) -> &[NL80211RegulatoryRule] {
        &self.rules
    }
}

bitflags::bitflags! {
    #[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
    pub struct NL80211RegulatoryRuleFlags : u32 {
        const NoOFDM		    = 1<<0;
        const NoCCK		        = 1<<1;
        const NoIndoor		    = 1<<2;
        const NoOutdoor         = 1<<3;
        const DFS			    = 1<<4;
        const PTPOnly		    = 1<<5;
        const PTMPOnly		    = 1<<6;
        const NoIR		        = 1<<7;
        const NoIBSS		    = 1<<8;
        const AutoBandwidth		= 1<<11;
        const IRConcurrent	    = 1<<12;
        const NoHT40Minus	    = 1<<13;
        const NoHT40Plus        = 1<<14;
        const No80MHZ		    = 1<<15;
        const No160MHZ		    = 1<<16;

        const _ = !0;
    }
}

impl From<u32> for NL80211RegulatoryRuleFlags {
    fn from(value: u32) -> Self {
        Self::from_bits_retain(value)
    }
}

impl From<NL80211RegulatoryRuleFlags> for u32 {
    fn from(value: NL80211RegulatoryRuleFlags) -> Self {
        value.bits()
    }
}

#[derive(Debug, Clone)]
pub struct NL80211RegulatoryRule {
    pub flags: NL80211RegulatoryRuleFlags,

    pub start_freq_khz: u32,
    pub end_freq_khz: u32,
    pub max_bandwidth_khz: u32,

    pub max_antenna_gain_mbi: Option<u32>,
    pub max_eirp_mbm: u32,

    pub dfs_cac_time: Option<u32>,
}

impl NL80211RegulatoryRule {
    pub fn from_nlas<T: AsRef<[u8]> + ?Sized>(nlas: NlasIterator<&T>) -> Result<Self, DecodeError> {
        let mut flags = Option::<NL80211RegulatoryRuleFlags>::None;
        let mut start_freq_khz = Option::<u32>::None;
        let mut end_freq_khz = Option::<u32>::None;
        let mut max_bandwidth_khz = Option::<u32>::None;
        let mut max_antenna_gain_mbi = Option::<u32>::None;
        let mut max_eirp_mbm = Option::<u32>::None;
        let mut dfs_cac_time = Option::<u32>::None;

        //Parse NLAs
        macro_rules! parse_attr {
                ($nla:expr $(, $attr:ident => $name:ident)*) => {
                    match $nla {
                        $(
                            RegRuleAttribute::$attr(v) => {
                                if $name.is_some() {
                                    return Err(DecodeError::from(concat!(
                                        "duplicate regulatory rule attribute: ",
                                        stringify!($attr)
                                    )));
                                }
                                $name = Some(v);
                            }
                        )*
                        RegRuleAttribute::Unknown(_) => {}
                    }
                };
            }

        for nla in nlas {
            let nla = nla?;
            let nla = RegRuleAttribute::parse(&nla)?;

            parse_attr!(
                nla,
                RuleFlags => flags,
                FreqRangeStart => start_freq_khz,
                FreqRangeEnd => end_freq_khz,
                FreqRangeMaxBandwidth => max_bandwidth_khz,
                PowerMaxAntennaGain => max_antenna_gain_mbi,
                PowerMaxEIRP => max_eirp_mbm,
                DFSCACTime => dfs_cac_time
            );
        }

        if dfs_cac_time == Some(0) {
            dfs_cac_time = None;
        }

        //Construct the rule
        macro_rules! require_attr {
            ($name: ident) => {
                let $name = $name.ok_or(DecodeError::from(concat!(
                    "missing required regulatory rule attribute: ",
                    stringify!($attr)
                )))?;
            };
        }
        require_attr!(start_freq_khz);
        require_attr!(end_freq_khz);
        require_attr!(max_bandwidth_khz);
        require_attr!(max_eirp_mbm);
        require_attr!(flags);

        Ok(NL80211RegulatoryRule {
            start_freq_khz,
            end_freq_khz,
            max_bandwidth_khz,
            max_antenna_gain_mbi,
            max_eirp_mbm,
            dfs_cac_time,
            flags,
        })
    }

    fn nlas(&self) -> ([RegRuleAttribute; 7], usize) {
        let mut attr_buf: [RegRuleAttribute; 7] = unsafe { std::mem::zeroed() };
        let mut attr_idx = 0;

        macro_rules! emit_attr {
            ($attr:ident, $val:expr) => {{
                attr_buf[attr_idx] = RegRuleAttribute::$attr($val);
                attr_idx += 1;
            }};
        }

        emit_attr!(RuleFlags, self.flags);
        emit_attr!(FreqRangeStart, self.start_freq_khz);
        emit_attr!(FreqRangeEnd, self.end_freq_khz);
        emit_attr!(FreqRangeMaxBandwidth, self.max_bandwidth_khz);
        if let Some(gain) = self.max_antenna_gain_mbi {
            emit_attr!(PowerMaxAntennaGain, gain);
        }
        emit_attr!(PowerMaxEIRP, self.max_eirp_mbm);
        if let Some(time) = self.dfs_cac_time {
            emit_attr!(DFSCACTime, time);
        }

        (attr_buf, attr_idx)
    }
}

impl Nla for NL80211RegulatoryRule {
    fn kind(&self) -> u16 {
        0
    }

    fn value_len(&self) -> usize {
        let (attr_buf, num_attrs) = self.nlas();
        (&attr_buf[..num_attrs]).buffer_len()
    }

    fn emit_value(&self, buffer: &mut [u8]) {
        let (attr_buf, num_attrs) = self.nlas();
        (&attr_buf[..num_attrs]).emit(buffer);
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> Parseable<NlaBuffer<&'a T>> for NL80211RegulatoryRule {
    fn parse(buf: &NlaBuffer<&'a T>) -> Result<Self, DecodeError> {
        Self::from_nlas(NlasIterator::new(buf.value()))
    }
}

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive)]
enum RegRuleAttributeTag {
    RuleFlags = 1,
    FreqRangeStart,
    FreqRangeEnd,
    FreqRangeMaxBandwidth,
    PowerMaxAntennaGain,
    PowerMaxEIRP,
    DFSCACTime,
}

#[allow(unused)]
#[derive(Debug, PartialEq, Eq, Clone)]
enum RegRuleAttribute {
    RuleFlags(NL80211RegulatoryRuleFlags),
    FreqRangeStart(u32),
    FreqRangeEnd(u32),
    FreqRangeMaxBandwidth(u32),
    PowerMaxAntennaGain(u32),
    PowerMaxEIRP(u32),
    DFSCACTime(u32),

    Unknown(DefaultNla),
}

impl Nla for RegRuleAttribute {
    fn value_len(&self) -> usize {
        match &self {
            Self::Unknown(nla) => nla.value_len(),
            _ => attr_size!(
                RegRuleAttribute,
                &self,
                RuleFlags => u32,
                FreqRangeStart => u32,
                FreqRangeEnd => u32,
                FreqRangeMaxBandwidth => u32,
                PowerMaxAntennaGain => u32,
                PowerMaxEIRP => u32,
                DFSCACTime => u32
            ),
        }
    }

    fn kind(&self) -> u16 {
        match &self {
            Self::Unknown(nla) => nla.kind(),
            _ => attr_tag!(
                RegRuleAttribute,
                RegRuleAttributeTag,
                &self,
                RuleFlags(_),
                FreqRangeStart(_),
                FreqRangeEnd(_),
                FreqRangeMaxBandwidth(_),
                PowerMaxAntennaGain(_),
                PowerMaxEIRP(_),
                DFSCACTime(_)
            ) as u16,
        }
    }

    fn emit_value(&self, buf: &mut [u8]) {
        match &self {
            Self::Unknown(nla) => nla.emit_value(buf),
            _ => emit_attr!(RegRuleAttribute, &self, buf,
                RuleFlags => (NL80211RegulatoryRuleFlags as Into<u32>),
                FreqRangeStart => u32,
                FreqRangeEnd => u32,
                FreqRangeMaxBandwidth => u32,
                PowerMaxAntennaGain => u32,
                PowerMaxEIRP => u32,
                DFSCACTime => u32
            ),
        }
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> Parseable<NlaBuffer<&'a T>> for RegRuleAttribute {
    fn parse(buf: &NlaBuffer<&'a T>) -> Result<Self, DecodeError> {
        let Ok(tag) = RegRuleAttributeTag::try_from(buf.kind()) else {
            return Ok(RegRuleAttribute::Unknown(DefaultNla::parse(buf)?));
        };

        Ok(parse_attr!(RegRuleAttribute, RegRuleAttributeTag, tag, buf,
            RuleFlags => (NL80211RegulatoryRuleFlags as From<u32>),
            FreqRangeStart => u32,
            FreqRangeEnd => u32,
            FreqRangeMaxBandwidth => u32,
            PowerMaxAntennaGain => u32,
            PowerMaxEIRP => u32,
            DFSCACTime => u32
        ))
    }
}

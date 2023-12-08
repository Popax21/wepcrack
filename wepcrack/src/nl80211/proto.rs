use netlink_packet_generic::{GenlFamily, GenlHeader};
use netlink_packet_utils::{
    nla::{Nla, NlasIterator},
    DecodeError, Emitable, Parseable, ParseableParametrized,
};
use num_enum::TryFromPrimitive;

use super::{NL80211Attribute, NL80211AttributeTag};

pub const NL80211_FAMILY_ID: &str = "nl80211";

#[repr(u8)]
#[derive(Clone, Copy, Debug, TryFromPrimitive, PartialEq, Eq)]
#[allow(unused)]
pub enum NL80211Command {
    Unspec,

    GetWiphy,
    SetWiphy,
    NewWiphy,
    DelWiphy,

    GetInterface,
    SetInterface,
    NewInterface,
    DelInterface,
}

#[derive(Clone, Debug)]
pub struct NL80211Message {
    pub cmd: NL80211Command,
    pub nlas: Vec<NL80211Attribute>,
}

impl NL80211Message {
    pub fn find_attr(&self, attr_tag: NL80211AttributeTag) -> Option<&NL80211Attribute> {
        self.nlas.iter().find(|attr| attr.kind() == attr_tag as u16)
    }
}

impl GenlFamily for NL80211Message {
    fn family_name() -> &'static str {
        NL80211_FAMILY_ID
    }

    fn command(&self) -> u8 {
        self.cmd as u8
    }

    fn version(&self) -> u8 {
        1
    }
}

impl Emitable for NL80211Message {
    fn buffer_len(&self) -> usize {
        self.nlas.as_slice().buffer_len()
    }

    fn emit(&self, buffer: &mut [u8]) {
        self.nlas.as_slice().emit(buffer)
    }
}

impl ParseableParametrized<[u8], GenlHeader> for NL80211Message {
    fn parse_with_param(buf: &[u8], header: GenlHeader) -> Result<NL80211Message, DecodeError> {
        Ok(NL80211Message {
            cmd: NL80211Command::try_from(header.cmd).map_err(|_| {
                DecodeError::from(format!("unknown nl80211 cmd: 0x{:2x}", header.cmd))
            })?,
            nlas: NlasIterator::new(buf)
                .map(|nla| NL80211Attribute::parse(&nla?))
                .collect::<Result<Vec<_>, _>>()?,
        })
    }
}

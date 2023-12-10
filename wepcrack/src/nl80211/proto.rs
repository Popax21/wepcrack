use core::panic;

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

    GetReg = 31,
    SetReg = 26,
}

#[derive(Clone, Debug)]
pub struct NL80211Message {
    pub cmd: NL80211Command,
    pub nlas: Vec<NL80211Attribute>,
}

impl NL80211Message {
    pub fn verify_cmd(&self, cmd: NL80211Command) {
        if self.cmd != cmd {
            panic!(
                "unexpected nl80211 message command: expected {cmd:?}, got {:?}",
                self.cmd
            );
        }
    }

    pub fn steal_attribute(&mut self, attr_tag: NL80211AttributeTag) -> Option<NL80211Attribute> {
        let Some(idx) = self
            .nlas
            .iter()
            .position(|attr| attr.kind() == attr_tag as u16)
        else {
            return None;
        };

        Some(std::mem::replace(
            &mut self.nlas[idx],
            NL80211Attribute::Unspec,
        ))
    }

    pub fn steal_required_attribute(&mut self, attr_tag: NL80211AttributeTag) -> NL80211Attribute {
        let Some(attr) = self.steal_attribute(attr_tag) else {
            panic!("nl80211 message lacks required attribute: {attr_tag:?}");
        };
        attr
    }
}

#[macro_export]
macro_rules! steal_msg_attr {
    ($tag:ident($name:ident) = $msg:expr) => {
        let NL80211Attribute::$tag($name) =
            $msg.steal_required_attribute(NL80211AttributeTag::$tag)
        else {
            unreachable!();
        };
    };
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

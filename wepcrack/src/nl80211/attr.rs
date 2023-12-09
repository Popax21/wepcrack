use netlink_packet_utils::{
    nla::{DefaultNla, Nla, NlaBuffer},
    DecodeError, Parseable,
};
use num_enum::TryFromPrimitive;

use super::{
    attr_macro::{attr_size, attr_tag, emit_attr, parse_attr},
    NL80211InterfaceType, NL80211WiphyIndex,
};

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive)]
pub enum NL80211AttributeTag {
    Unspec = 0,

    WiphyIndex = 1,
    WiphyName = 2,

    InterfaceIndex = 3,
    InterfaceName = 4,
    InterfaceType = 5,
    SupportedInterfaceTypes = 32,

    MacAddress = 6,
}

#[allow(unused)]
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum NL80211Attribute {
    Unknown(DefaultNla),
    Unspec,

    WiphyIndex(NL80211WiphyIndex),
    WiphyName(String),

    InterfaceIndex(u32),
    InterfaceName(String),
    InterfaceType(NL80211InterfaceType),
    SupportedInterfaceTypes(Vec<NL80211InterfaceType>),

    MacAddress([u8; 6]),
}

impl Nla for NL80211Attribute {
    fn value_len(&self) -> usize {
        match &self {
            Self::Unknown(nla) => nla.value_len(),
            _ => attr_size!(NL80211Attribute, &self,
                Unspec => (),

                WiphyIndex => u32,
                WiphyName => String,

                InterfaceIndex => u32,
                InterfaceName => String,
                InterfaceType => (enum NL80211InterfaceType(u32)),
                SupportedInterfaceTypes => [(enum NL80211InterfaceType(<kind>))],

                MacAddress => [u8; 6]
            ),
        }
    }

    fn kind(&self) -> u16 {
        match &self {
            Self::Unknown(nla) => nla.kind(),
            _ => attr_tag!(
                NL80211Attribute,
                NL80211AttributeTag,
                &self,
                Unspec,
                WiphyIndex(_),
                WiphyName(_),
                InterfaceIndex(_),
                InterfaceName(_),
                InterfaceType(_),
                SupportedInterfaceTypes(_),
                MacAddress(_)
            ) as u16,
        }
    }

    fn emit_value(&self, buf: &mut [u8]) {
        match &self {
            Self::Unknown(nla) => nla.emit_value(buf),
            _ => emit_attr!(NL80211Attribute, &self, buf,
                Unspec => (),

                WiphyIndex => u32,
                WiphyName => String,

                InterfaceIndex => u32,
                InterfaceName => String,
                InterfaceType => (enum NL80211InterfaceType(u32)),
                SupportedInterfaceTypes => [(enum NL80211InterfaceType(<kind>))],

                MacAddress => [u8; 6]
            ),
        }
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> Parseable<NlaBuffer<&'a T>> for NL80211Attribute {
    fn parse(buf: &NlaBuffer<&'a T>) -> Result<Self, DecodeError> {
        let Ok(tag) = NL80211AttributeTag::try_from(buf.kind()) else {
            return Ok(NL80211Attribute::Unknown(DefaultNla::parse(buf)?));
        };

        Ok(parse_attr!(NL80211Attribute, NL80211AttributeTag, tag, buf,
            Unspec => (),

            WiphyIndex => u32,
            WiphyName => String,

            InterfaceIndex => u32,
            InterfaceName => String,
            InterfaceType => (enum NL80211InterfaceType((u32 as u16))),
            SupportedInterfaceTypes => [(enum NL80211InterfaceType(<kind>))],

            MacAddress => [u8; 6]
        ))
    }
}

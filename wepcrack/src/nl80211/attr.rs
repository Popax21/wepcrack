use netlink_packet_utils::{
    byteorder::{ByteOrder, NativeEndian},
    nla::{DefaultNla, Nla, NlaBuffer},
    parsers::{parse_string, parse_u32},
    DecodeError, Parseable,
};
use num_enum::TryFromPrimitive;

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive)]
pub enum NL80211AttributeTag {
    Unspec,

    Whipy,
    WhipyName,

    InterfaceIndex,
    InterfaceName,

    MAC,
}

#[allow(unused)]
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum NL80211Attribute {
    Unspec,

    Whipy(u32),
    WhipyName(String),

    InterfaceIndex(u32),
    InterfaceName(String),

    MAC([u8; 6]),

    Unknown(DefaultNla),
}

impl Nla for NL80211Attribute {
    fn value_len(&self) -> usize {
        match &self {
            NL80211Attribute::Unspec => 0,

            //u32 attributes
            NL80211Attribute::Whipy(_) | NL80211Attribute::InterfaceIndex(_) => {
                std::mem::size_of::<u32>()
            }

            //string attributes
            NL80211Attribute::WhipyName(s) | NL80211Attribute::InterfaceName(s) => s.len() + 1,

            //special attributes
            NL80211Attribute::MAC(_) => 6,

            NL80211Attribute::Unknown(nla) => nla.value_len(),
        }
    }

    fn kind(&self) -> u16 {
        match &self {
            NL80211Attribute::Unspec => NL80211AttributeTag::Unspec as u16,
            NL80211Attribute::Whipy(_) => NL80211AttributeTag::Whipy as u16,
            NL80211Attribute::WhipyName(_) => NL80211AttributeTag::WhipyName as u16,
            NL80211Attribute::InterfaceIndex(_) => NL80211AttributeTag::InterfaceIndex as u16,
            NL80211Attribute::InterfaceName(_) => NL80211AttributeTag::InterfaceName as u16,
            NL80211Attribute::MAC(_) => NL80211AttributeTag::MAC as u16,

            NL80211Attribute::Unknown(nla) => nla.kind(),
        }
    }

    fn emit_value(&self, buffer: &mut [u8]) {
        match &self {
            NL80211Attribute::Unspec => {}

            //u32 attributes
            NL80211Attribute::Whipy(v) | NL80211Attribute::InterfaceIndex(v) => {
                NativeEndian::write_u32(buffer, *v)
            }

            //string attributes
            NL80211Attribute::WhipyName(s) | NL80211Attribute::InterfaceName(s) => {
                buffer[..s.len()].copy_from_slice(s.as_bytes());
                buffer[s.len()] = 0;
            }

            //special attributes
            NL80211Attribute::MAC(mac) => buffer[..6].copy_from_slice(mac),

            NL80211Attribute::Unknown(nla) => nla.emit_value(buffer),
        }
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> Parseable<NlaBuffer<&'a T>> for NL80211Attribute {
    fn parse(buf: &NlaBuffer<&'a T>) -> Result<Self, DecodeError> {
        let Ok(tag) = NL80211AttributeTag::try_from(buf.kind()) else {
            return Ok(NL80211Attribute::Unknown(DefaultNla::parse(buf)?));
        };

        let check_buffer_len = |len: usize| -> Result<(), DecodeError> {
            if buf.value_length() == len {
                Ok(())
            } else {
                Err(DecodeError::from(
                    "unexpected nl80211 attribute payload length",
                ))
            }
        };

        Ok(match tag {
            NL80211AttributeTag::Unspec => {
                check_buffer_len(0)?;
                NL80211Attribute::Unspec
            }

            NL80211AttributeTag::Whipy => NL80211Attribute::Whipy(parse_u32(buf.value())?),
            NL80211AttributeTag::WhipyName => {
                NL80211Attribute::WhipyName(parse_string(buf.value())?)
            }

            NL80211AttributeTag::InterfaceIndex => {
                NL80211Attribute::InterfaceIndex(parse_u32(buf.value())?)
            }
            NL80211AttributeTag::InterfaceName => {
                NL80211Attribute::InterfaceName(parse_string(buf.value())?)
            }

            NL80211AttributeTag::MAC => NL80211Attribute::MAC({
                check_buffer_len(6)?;

                let mut mac = [0u8; 6];
                mac.copy_from_slice(buf.value());
                mac
            }),
        })
    }
}

macro_rules! val_size {
    //primitives
    ((), $val:expr) => {
        0
    };
    (u16, $val:expr) => {
        std::mem::size_of::<u16>()
    };
    (u32, $val:expr) => {
        std::mem::size_of::<u32>()
    };
    (String, $val:expr) => {{
        $val.len() + 1
    }};

    //composite
    ((enum $enum:ident($type:tt)), $val:expr) => {
        $crate::nl80211::attr_macro::val_size!($type, &(*$val as $type))
    };

    ([(enum $enum:ident(<kind>))], $val:expr) => {
        netlink_packet_utils::nla::NLA_HEADER_SIZE * $val.len()
    };

    ([u8; $num:literal], $val:expr) => {
        $num
    };
}
pub(super) use val_size;

macro_rules! emit_val {
    //primitives
    ((), $val:expr, $buf:expr) => {{}};

    (u16, $val:expr, $buf:expr) => {{
        use netlink_packet_utils::byteorder::ByteOrder;
        netlink_packet_utils::byteorder::NativeEndian::write_u16($buf, *$val)
    }};
    (u32, $val:expr, $buf:expr) => {{
        use netlink_packet_utils::byteorder::ByteOrder;
        netlink_packet_utils::byteorder::NativeEndian::write_u32($buf, *$val)
    }};
    (String, $val:expr, $buf:expr) => {{
        use std::ops::Deref;

        let s: &str = $val.deref();
        $buf[..s.len()].copy_from_slice(s.as_bytes());
        $buf[s.len()] = 0;
    }};

    //composite
    ((enum $enum:ident($type:tt)), $val:expr, $buf:expr) => {
        $crate::nl80211::attr_macro::emit_val!($type, &(*$val as $type), $buf)
    };

    ([(enum $enum:ident(<kind>))], $val:expr, $buf:expr) => {{
        for (i, v) in $val.iter().enumerate() {
            let mut nlabuf = netlink_packet_utils::nla::NlaBuffer::<&mut [u8]>::new(&mut $buf[i..]);
            nlabuf.set_kind(*v as u16);
            nlabuf.set_length(0);
        }
    }};

    ([u8; $num:literal], $val:expr, $buf:expr) => {
        $buf[..$num].copy_from_slice($val)
    };
}
pub(super) use emit_val;

macro_rules! parse_val {
    //primitives
    (u16, $buf:expr) => {
        netlink_packet_utils::parsers::parse_u16($buf.value())?
    };
    (u32, $buf:expr) => {
        netlink_packet_utils::parsers::parse_u32($buf.value())?
    };
    (String, $buf:expr) => {
        netlink_packet_utils::parsers::parse_string($buf.value())?
    };

    //composite
    (($type:tt as $cast:tt), $buf:expr) => {
        $crate::nl80211::attr_macro::parse_val!($type, $buf) as $cast
    };
    ((enum $enum:ident($type:tt)), $buf:expr) => {{
        let val = $crate::nl80211::attr_macro::parse_val!($type, $buf);
        $enum::try_from(val)
            .map_err(|_| DecodeError::from(format!("unexpected nl80211 enum value: {val:?}")))?
    }};
    ((enum $enum:ident(<kind>)), $buf:expr) => {{
        $enum::try_from($buf.kind()).map_err(|_| {
            DecodeError::from(format!("unexpected nl80211 enum value: {:?}", $buf.kind()))
        })?
    }};

    ([$type:tt], $buf:expr) => {
        netlink_packet_utils::nla::NlasIterator::new($buf.value())
            .map(|res| res.and_then(|nla| Ok($crate::nl80211::attr_macro::parse_val!($type, nla))))
            .collect::<Result<Vec<_>, _>>()?
    };

    ([u8; $num:literal], $buf:expr) => {{
        $crate::nl80211::attr_macro::check_nla_payload_size!($buf, $num);

        let mut val = [0u8; $num];
        val.copy_from_slice($buf.value());
        val
    }};
}
pub(super) use parse_val;

macro_rules! check_nla_payload_size {
    ($buf:expr, $len:expr) => {
        if $buf.value_length() != $len {
            return Err(DecodeError::from(format!(
                "unexpected nl80211 attribute payload length: expected {}, got {}",
                $buf.value_length(),
                $len
            )));
        }
    };
}
pub(super) use check_nla_payload_size;

macro_rules! attr_matcher {
    ($attr_type:ident, $tag:ident(()) => $val:ident) => {
        $attr_type::$tag
    };
    ($attr_type:ident, $tag:ident($emit:tt) => $val:ident) => {
        $attr_type::$tag($val)
    };
}
pub(super) use attr_matcher;

macro_rules! attr_tag {
    ($attr_type:ident, $tag_type:ident, $attr:expr $(, $tag:ident$(($matcher:tt))?)*) => {
        match $attr {
            $( $attr_type::$tag$(($matcher))? => $tag_type::$tag, )*
            _ => unreachable!()
        }
    };
}
pub(super) use attr_tag;

macro_rules! attr_size {
    ($attr_type:ident, $attr:expr $(, $tag:ident => $emit:tt)*) => {
        match $attr {
            $( $crate::nl80211::attr_macro::attr_matcher!($attr_type, $tag($emit) => _v) => $crate::nl80211::attr_macro::val_size!($emit, _v), )*
            _ => unreachable!()
        }
    };
}
pub(super) use attr_size;

macro_rules! emit_attr {
    ($attr_type:ident, $attr:expr, $buf:expr $(, $tag:ident => $emit:tt)*) => {
        match $attr {
            $( $crate::nl80211::attr_macro::attr_matcher!($attr_type, $tag($emit) => _v) => $crate::nl80211::attr_macro::emit_val!($emit, _v, $buf), )*
            _ => unreachable!()
        }
    };
}
pub(super) use emit_attr;

macro_rules! parse_attr {
    ($attr_type:ident, $tag_type:ident, $tag:expr, $buf:expr $(, $opt_tag:ident => $parse:tt)*) => {
        match $tag {
            $( $tag_type::$opt_tag => parse_attr!($attr_type::$opt_tag => $parse, $buf), )*
        }
    };
    ($attr_type:ident::$tag:ident => (), $buf:expr) => {
        {
            $crate::nl80211::attr_macro::check_nla_payload_size!($buf, 0);
            $attr_type::$tag
        }
    };
    ($attr_type:ident::$tag:ident => $type:tt, $buf:expr) => { $attr_type::$tag($crate::nl80211::attr_macro::parse_val!($type, $buf)) };
}
pub(super) use parse_attr;

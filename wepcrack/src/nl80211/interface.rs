use num_enum::TryFromPrimitive;

use crate::steal_msg_attr;

use super::{
    NL80211Attribute, NL80211AttributeTag, NL80211Command, NL80211Connection, NL80211Message,
    NL80211Wiphy, NL80211WiphyIndex,
};

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive)]
pub enum NL80211InterfaceType {
    Unspecified,
    Adhoc,
    Station,
    Ap,
    ApVLAN,
    Wds,
    Monitor,
    MeshPoint,
    P2PClient,
    P2PGroupOwner,
    P2PDevice,
    Ocb,
    Nan,
}

pub type NL80211InterfaceIndex = u32;

#[derive(Debug, Clone)]
pub struct NL80211Interface {
    index: NL80211InterfaceIndex,

    name: String,
    interface_type: NL80211InterfaceType,
    mac_address: [u8; 6],
    wiphy: NL80211WiphyIndex,
}

impl NL80211Interface {
    pub fn from_message(mut msg: NL80211Message) -> NL80211Interface {
        msg.verify_cmd(NL80211Command::NewInterface);
        steal_msg_attr!(InterfaceIndex(index) = msg);
        steal_msg_attr!(InterfaceName(name) = msg);
        steal_msg_attr!(InterfaceType(interface_type) = msg);
        steal_msg_attr!(MacAddress(mac_address) = msg);
        steal_msg_attr!(WiphyIndex(wiphy) = msg);

        NL80211Interface {
            index,
            name,
            interface_type,
            mac_address,
            wiphy,
        }
    }

    pub fn from_index(
        con: &NL80211Connection,
        idx: NL80211WiphyIndex,
    ) -> anyhow::Result<NL80211Interface> {
        Ok(Self::from_message(con.send_get_request(
            NL80211Message {
                cmd: NL80211Command::GetInterface,
                nlas: vec![NL80211Attribute::InterfaceIndex(idx)],
            },
        )?))
    }

    pub fn query_list(con: &NL80211Connection) -> anyhow::Result<Vec<NL80211Interface>> {
        Ok(con
            .send_dump_request(NL80211Message {
                cmd: NL80211Command::GetInterface,
                nlas: vec![],
            })?
            .into_iter()
            .map(Self::from_message)
            .collect())
    }

    pub fn create_new(
        con: &NL80211Connection,
        wiphy: &NL80211Wiphy,
        name: &str,
        interface_type: NL80211InterfaceType,
    ) -> anyhow::Result<NL80211Interface> {
        Ok(Self::from_message(con.send_get_request(
            NL80211Message {
                cmd: NL80211Command::NewInterface,
                nlas: vec![
                    NL80211Attribute::WiphyIndex(wiphy.index()),
                    NL80211Attribute::InterfaceName(name.to_owned()),
                    NL80211Attribute::InterfaceType(interface_type),
                ],
            },
        )?))
    }

    pub const fn index(&self) -> NL80211InterfaceIndex {
        self.index
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn interface_type(&self) -> NL80211InterfaceType {
        self.interface_type
    }

    pub const fn mac_address(&self) -> &[u8; 6] {
        &self.mac_address
    }

    pub const fn wiphy(&self) -> NL80211WiphyIndex {
        self.wiphy
    }

    pub fn delete(&self, con: &NL80211Connection) -> anyhow::Result<()> {
        con.send_acked_request(NL80211Message {
            cmd: NL80211Command::DelInterface,
            nlas: vec![
                NL80211Attribute::WiphyIndex(self.wiphy),
                NL80211Attribute::InterfaceIndex(self.index),
            ],
        })?;

        Ok(())
    }
}

use num_enum::TryFromPrimitive;

use crate::steal_msg_attr;

use super::{
    NL80211Attribute, NL80211AttributeTag, NL80211Channel, NL80211Command, NL80211Connection,
    NL80211Message, NL80211Wiphy, NL80211WiphyIndex,
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
    pub fn from_message(mut msg: NL80211Message) -> Option<NL80211Interface> {
        msg.verify_cmd(NL80211Command::NewInterface);

        let Some(NL80211Attribute::InterfaceIndex(index)) =
            msg.steal_attribute(NL80211AttributeTag::InterfaceIndex)
        else {
            return None;
        };

        steal_msg_attr!(InterfaceName(name) = msg);
        steal_msg_attr!(InterfaceType(interface_type) = msg);
        steal_msg_attr!(MacAddress(mac_address) = msg);
        steal_msg_attr!(WiphyIndex(wiphy) = msg);

        Some(NL80211Interface {
            index,
            name,
            interface_type,
            mac_address,
            wiphy,
        })
    }

    pub fn from_index(
        con: &NL80211Connection,
        idx: NL80211WiphyIndex,
    ) -> anyhow::Result<NL80211Interface> {
        Self::from_message(con.send_get_request(NL80211Message {
            cmd: NL80211Command::GetInterface,
            nlas: vec![NL80211Attribute::InterfaceIndex(idx)],
        })?)
        .ok_or(anyhow::anyhow!(
            "nl80211 interface with index {idx} is not a valid interface"
        ))
    }

    pub fn query_list(con: &NL80211Connection) -> anyhow::Result<Vec<NL80211Interface>> {
        Ok(con
            .send_dump_request(NL80211Message {
                cmd: NL80211Command::GetInterface,
                nlas: vec![],
            })?
            .into_iter()
            .flat_map(Self::from_message)
            .collect())
    }

    pub fn create_new(
        con: &NL80211Connection,
        wiphy: &NL80211Wiphy,
        name: &str,
        interface_type: NL80211InterfaceType,
        con_owned: bool,
    ) -> anyhow::Result<NL80211Interface> {
        let mut nlas = vec![
            NL80211Attribute::WiphyIndex(wiphy.index()),
            NL80211Attribute::InterfaceName(name.to_owned()),
            NL80211Attribute::InterfaceType(interface_type),
        ];
        if con_owned {
            nlas.push(NL80211Attribute::SocketOwner)
        }

        Ok(Self::from_message(con.send_get_request(NL80211Message {
            cmd: NL80211Command::NewInterface,
            nlas,
        })?)
        .unwrap())
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

    pub fn get_channel(&self, con: &NL80211Connection) -> anyhow::Result<Option<NL80211Channel>> {
        let mut resp = con.send_get_request(NL80211Message {
            cmd: NL80211Command::GetInterface,
            nlas: vec![NL80211Attribute::InterfaceIndex(self.index)],
        })?;

        resp.verify_cmd(NL80211Command::NewInterface);

        if !resp.has_attribute(NL80211AttributeTag::WiphyFreq) {
            return Ok(None);
        }

        steal_msg_attr!(WiphyFreq(freq) = resp);
        steal_msg_attr!(ChannelWidth(width) = resp);

        let center_freq1 = resp
            .steal_attribute(NL80211AttributeTag::CenterFreq1)
            .map(|attr| {
                if let NL80211Attribute::CenterFreq1(freq) = attr {
                    freq
                } else {
                    unreachable!()
                }
            });

        let center_freq2 = resp
            .steal_attribute(NL80211AttributeTag::CenterFreq2)
            .map(|attr| {
                if let NL80211Attribute::CenterFreq2(freq) = attr {
                    freq
                } else {
                    unreachable!()
                }
            });

        NL80211Channel::new(freq, width, center_freq1, center_freq2)
            .ok_or(anyhow::anyhow!(
                "invalid or unsupported current interface channel"
            ))
            .map(Some)
    }

    pub fn set_channel(
        &self,
        channel: &NL80211Channel,
        con: &NL80211Connection,
    ) -> anyhow::Result<()> {
        let mut nlas = vec![
            NL80211Attribute::InterfaceIndex(self.index),
            NL80211Attribute::WiphyFreq(channel.frequency()),
            NL80211Attribute::ChannelWidth(channel.width()),
        ];

        if let Some(center_freq1) = channel.center_freq1() {
            nlas.push(NL80211Attribute::CenterFreq1(center_freq1));
        }
        if let Some(center_freq2) = channel.center_freq1() {
            nlas.push(NL80211Attribute::CenterFreq2(center_freq2));
        }

        con.send_acked_request(NL80211Message {
            cmd: NL80211Command::SetChannel,
            nlas,
        })
    }
}

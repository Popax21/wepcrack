use std::error::Error;

use crate::steal_msg_attr;

use super::{
    NL80211Attribute, NL80211AttributeTag, NL80211Command, NL80211Connection, NL80211InterfaceType,
    NL80211Message,
};

pub type NL80211WiphyIndex = u32;

#[derive(Debug, Clone)]
pub struct NL80211Wiphy {
    index: NL80211WiphyIndex,

    name: String,
    supported_interface_types: Vec<NL80211InterfaceType>,
}

impl NL80211Wiphy {
    fn from_message(mut msg: NL80211Message) -> NL80211Wiphy {
        steal_msg_attr!(WiphyIndex(index) = msg);
        steal_msg_attr!(WiphyName(name) = msg);
        steal_msg_attr!(SupportedInterfaceTypes(if_types) = msg);

        NL80211Wiphy {
            index,
            name,
            supported_interface_types: if_types,
        }
    }

    pub fn from_index(
        con: &NL80211Connection,
        idx: NL80211WiphyIndex,
    ) -> Result<NL80211Wiphy, Box<dyn Error>> {
        //Send a GET_WIPHY request
        con.send_request(
            NL80211Message {
                cmd: NL80211Command::GetWiphy,
                nlas: vec![NL80211Attribute::WiphyIndex(idx)],
            },
            false,
        )?;

        Ok(Self::from_message(
            con.recv_response(NL80211Command::NewWiphy)?,
        ))
    }

    pub fn query_list(con: &NL80211Connection) -> Result<Vec<NL80211Wiphy>, Box<dyn Error>> {
        //Send a dump GET_WIPHY request
        con.send_request(
            NL80211Message {
                cmd: NL80211Command::GetWiphy,
                nlas: vec![],
            },
            true,
        )?;

        let wiphys = con.recv_dump_response(NL80211Command::NewWiphy)?;
        Ok(wiphys.into_iter().map(Self::from_message).collect())
    }

    pub const fn index(&self) -> NL80211WiphyIndex {
        self.index
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn supported_interface_types(&self) -> &[NL80211InterfaceType] {
        &self.supported_interface_types
    }
}

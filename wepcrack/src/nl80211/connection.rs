use anyhow::Context;
use netlink_packet_core::{NLM_F_ACK, NLM_F_REQUEST};
use netlink_packet_generic::{
    ctrl::{nlas::GenlCtrlAttrs, GenlCtrl, GenlCtrlCmd},
    GenlMessage,
};
use netlink_sys::protocols::NETLINK_GENERIC;

use crate::{netlink::NetlinkConnection, netlink_req_funcs};

use super::{NL80211Message, NL80211_FAMILY_ID};

pub struct NL80211Connection {
    connection: NetlinkConnection,
    family_id: u16,
}

impl NL80211Connection {
    pub fn new() -> anyhow::Result<NL80211Connection> {
        //Create the netlink connection
        let connection = NetlinkConnection::new(NETLINK_GENERIC)?;

        //Resolve the nl80211 family ID
        let mut family_id = 0u16;
        let mut msg = GenlMessage::from_payload(GenlCtrl {
            cmd: GenlCtrlCmd::GetFamily,
            nlas: vec![GenlCtrlAttrs::FamilyName(NL80211_FAMILY_ID.to_owned())],
        });
        msg.finalize();

        connection
            .send_request(msg, NLM_F_REQUEST | NLM_F_ACK, |msg| {
                //Find the family ID NLA
                family_id = msg
                    .payload
                    .nlas
                    .iter()
                    .find_map(|nla| {
                        if let GenlCtrlAttrs::FamilyId(id) = nla {
                            Some(*id)
                        } else {
                            None
                        }
                    })
                    .expect("response to family ID query didn't contain a family ID NLA");

                Ok(())
            })
            .context("failed to resolve nl80211 family ID")?;

        Ok(NL80211Connection {
            connection,
            family_id,
        })
    }

    fn send_request(
        &self,
        msg: NL80211Message,
        header_flags: u16,
        mut resp_cb: impl FnMut(NL80211Message) -> anyhow::Result<()>,
    ) -> anyhow::Result<()> {
        let mut msg = GenlMessage::from_payload(msg);
        msg.set_resolved_family_id(self.family_id);
        msg.finalize();

        self.connection
            .send_request(msg, header_flags, |msg| resp_cb(msg.payload))
    }
}

netlink_req_funcs!(NL80211Connection, NL80211Message);

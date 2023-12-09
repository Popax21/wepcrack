use std::error::Error;

use netlink_packet_core::{
    NetlinkDeserializable, NetlinkMessage, NetlinkPayload, NetlinkSerializable, NLM_F_DUMP,
    NLM_F_REQUEST,
};
use netlink_packet_generic::{
    ctrl::{nlas::GenlCtrlAttrs, GenlCtrl, GenlCtrlCmd},
    GenlMessage,
};
use netlink_sys::{protocols::NETLINK_GENERIC, Socket, SocketAddr};

use super::{NL80211Command, NL80211Message, NL80211_FAMILY_ID};

const RX_BUFFER_SIZE: usize = 4096;
const TX_BUFFER_SIZE: usize = 4096;

pub struct NL80211Connection {
    socket: Socket,
    family_id: u16,
}

impl NL80211Connection {
    pub fn new() -> Result<NL80211Connection, Box<dyn Error>> {
        //Create and connect a new netlink socket
        let mut socket = Socket::new(NETLINK_GENERIC)?;
        socket.bind_auto()?;
        socket.connect(&SocketAddr::new(0, 0))?;

        //Setup the initial connection structure
        let mut con = NL80211Connection {
            socket,
            family_id: 0,
        };

        //Resolve the nl80211 family ID
        con.family_id = {
            //Send the request
            let mut msg = GenlMessage::from_payload(GenlCtrl {
                cmd: GenlCtrlCmd::GetFamily,
                nlas: vec![GenlCtrlAttrs::FamilyName(NL80211_FAMILY_ID.to_owned())],
            });
            msg.finalize();

            let mut msg = NetlinkMessage::from(msg);
            msg.header.flags = NLM_F_REQUEST;
            msg.finalize();

            con.send_message(&msg)?;

            //Await the response
            con.poll_messages::<GenlMessage<GenlCtrl>, u16>(|msg| {
                msg.payload.nlas.iter().find_map(|nla| {
                    if let GenlCtrlAttrs::FamilyId(id) = nla {
                        Some(*id)
                    } else {
                        None
                    }
                })
            })?
            .unwrap()
        };

        Ok(con)
    }
}

impl NL80211Connection {
    pub fn send_request(&self, msg: NL80211Message, dump: bool) -> Result<(), Box<dyn Error>> {
        //Serialize the message
        let mut msg = GenlMessage::from_payload(msg);
        msg.set_resolved_family_id(self.family_id);
        msg.finalize();

        let mut msg = NetlinkMessage::from(msg);
        msg.header.flags = NLM_F_REQUEST | if dump { NLM_F_DUMP } else { 0 };
        msg.finalize();

        let mut req_buf = [0u8; TX_BUFFER_SIZE];
        msg.serialize(&mut req_buf);
        let req_buf = &req_buf[..msg.buffer_len()];

        //Send it over the socket
        self.socket.send(req_buf, 0)?;

        Ok(())
    }

    pub fn recv_response(&self, cmd: NL80211Command) -> Result<NL80211Message, Box<dyn Error>> {
        if let Some(msg) =
            self.poll_messages::<GenlMessage<NL80211Message>, NL80211Message>(|msg| {
                if msg.payload.cmd == cmd {
                    Some(msg.payload)
                } else {
                    None
                }
            })?
        {
            Ok(msg)
        } else {
            Err("didn't receive an expected {cmd} response packet")?
        }
    }

    pub fn recv_dump_response(
        &self,
        cmd: NL80211Command,
    ) -> Result<Vec<NL80211Message>, Box<dyn Error>> {
        let mut msgs = Vec::<NL80211Message>::new();
        self.poll_messages::<GenlMessage<NL80211Message>, ()>(|msg| {
            if msg.payload.cmd == cmd {
                msgs.push(msg.payload);
            }
            None
        })?;

        Ok(msgs)
    }

    fn send_message(&self, msg: &NetlinkMessage<impl NetlinkSerializable>) -> std::io::Result<()> {
        let mut buf = [0u8; TX_BUFFER_SIZE];
        msg.serialize(&mut buf);
        self.socket.send(&buf[..msg.buffer_len()], 0).map(|_| ())
    }

    fn poll_messages<M: NetlinkDeserializable, T>(
        &self,
        mut cb: impl FnMut(M) -> Option<T>,
    ) -> Result<Option<T>, Box<dyn Error>> {
        let mut rx_buf = [0u8; RX_BUFFER_SIZE];

        loop {
            //Receive response data from the socket
            let rx_size = {
                let mut resp_buf = &mut rx_buf[..];
                self.socket.recv(&mut resp_buf, 0)?
            };
            let rx_buf = &rx_buf[..rx_size];

            //Parse response messages
            let mut off = 0usize;
            loop {
                //Parse the message
                let msg = <NetlinkMessage<M>>::deserialize(&rx_buf[off..])?;
                let msg_size = msg.header.length as usize;

                match msg.payload {
                    NetlinkPayload::Done(_) => return Ok(None),
                    NetlinkPayload::Error(err) => return Err(err.to_io())?,
                    NetlinkPayload::Noop => {}
                    NetlinkPayload::Overrun(_) => panic!("reached NetlinkPayload::Overrun handler"),
                    NetlinkPayload::InnerMessage(msg) => {
                        if let Some(res) = cb(msg) {
                            return Ok(Some(res));
                        }
                    }
                    _ => {}
                }

                //Move onto the next message
                off += msg_size;
                if msg_size == 0 || off >= rx_size {
                    break;
                }
            }
        }
    }
}

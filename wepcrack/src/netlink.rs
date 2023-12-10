use core::panic;
use std::sync::atomic::{AtomicU32, Ordering};

use anyhow::Context;
use netlink_packet_core::{
    ErrorBuffer, NetlinkBuffer, NetlinkDeserializable, NetlinkHeader, NetlinkMessage,
    NetlinkPayload, NetlinkSerializable, NLMSG_DONE, NLMSG_ERROR, NLMSG_NOOP, NLMSG_OVERRUN,
};
use netlink_packet_utils::Parseable;
use netlink_sys::{Socket, SocketAddr};

const RX_BUFFER_SIZE: usize = 4096;
const TX_BUFFER_SIZE: usize = 4096;

pub struct NetlinkConnection {
    socket: Socket,
    next_seq_number: AtomicU32,
}

impl NetlinkConnection {
    pub fn new(protocol: isize) -> anyhow::Result<NetlinkConnection> {
        //Create and connect a new netlink socket
        let mut socket = Socket::new(protocol)?;
        socket.bind_auto()?;
        socket.connect(&SocketAddr::new(0, 0))?;

        //Setup the initial connection structure
        Ok(NetlinkConnection {
            socket,
            next_seq_number: AtomicU32::new(0),
        })
    }
}

impl NetlinkConnection {
    pub fn send_request<
        T: Into<NetlinkPayload<T>> + NetlinkSerializable + NetlinkDeserializable,
    >(
        &self,
        msg: T,
        header_flags: u16,
        mut resp_cb: impl FnMut(T) -> anyhow::Result<()>,
    ) -> anyhow::Result<()> {
        //Send the message
        let seq = self
            .send_message(msg, header_flags)
            .context("failed to send request message")?;

        //Poll responses
        self.poll_response(seq, |msg_buf| {
            //Parse the response
            let msg_header =
                NetlinkHeader::parse(msg_buf).context("failed to parse response message header")?;
            let msg = T::deserialize(&msg_header, msg_buf.payload())
                .context("failed to parse response message")?;

            //Forward it to the callback
            resp_cb(msg)
        })
        .context("error while polling response messages")
    }

    fn send_message<T: Into<NetlinkPayload<T>> + NetlinkSerializable>(
        &self,
        msg: T,
        header_flags: u16,
    ) -> std::io::Result<u32> {
        //Prepare the message
        let mut msg = NetlinkMessage::from(msg);
        msg.header.flags = header_flags;
        msg.header.sequence_number = self.next_seq_number.fetch_add(1, Ordering::Relaxed);
        msg.finalize();

        //Serialize the message and send it
        let mut buf = [0u8; TX_BUFFER_SIZE];
        msg.serialize(&mut buf);
        self.socket.send(&buf[..msg.buffer_len()], 0).map(|_| ())?;

        Ok(msg.header.sequence_number)
    }

    fn poll_response(
        &self,
        seq: u32,
        mut cb: impl FnMut(&NetlinkBuffer<&[u8]>) -> anyhow::Result<()>,
    ) -> anyhow::Result<()> {
        let mut rx_buf = [0u8; RX_BUFFER_SIZE];

        loop {
            //Receive response data from the socket
            let rx_size = {
                let mut resp_buf = &mut rx_buf[..];
                self.socket
                    .recv(&mut resp_buf, 0)
                    .context("failed to receive response messages from socket")?
            };
            let rx_buf = &rx_buf[..rx_size];

            //Parse response messages
            let mut off = 0usize;
            loop {
                //Parse the message
                let msg_buf = NetlinkBuffer::new_checked(&rx_buf[off..])
                    .context("failed to create buffer for response message")?;
                if msg_buf.sequence_number() == seq {
                    //Handle the message
                    match msg_buf.message_type() {
                        NLMSG_NOOP => {}
                        NLMSG_ERROR => {
                            let err_buf = ErrorBuffer::new_checked(msg_buf.payload())
                                .context("failed to parse netlink error response")?;
                            return if let Some(err_code) = err_buf.code() {
                                //NAK
                                Err(std::io::Error::from_raw_os_error(err_code.get().abs()))
                                    .context("received NAK error response")?
                            } else {
                                //ACK
                                Ok(())
                            };
                        }
                        NLMSG_DONE => return Ok(()),
                        NLMSG_OVERRUN => {
                            panic!("reached NLMSG_OVERRUN handler")
                        }
                        _ => cb(&msg_buf).context("error while handling response message")?,
                    }
                }

                //Move onto the next message
                let msg_size = msg_buf.length() as usize;
                off += msg_size;
                if msg_size == 0 || off >= rx_size {
                    break;
                }
            }
        }
    }
}

#[macro_export]
macro_rules! netlink_req_funcs {
    ($con_type:tt, $msg_type:tt) => {
        impl $con_type {
            pub fn send_acked_request(&self, msg: $msg_type) -> anyhow::Result<()> {
                self.send_request(
                    msg,
                    netlink_packet_core::NLM_F_REQUEST | netlink_packet_core::NLM_F_ACK,
                    |msg| {
                        Err(anyhow::anyhow!(
                            "received response message to acked query request: {msg:?}"
                        ))
                    },
                )
            }

            pub fn send_get_request(&self, msg: $msg_type) -> anyhow::Result<$msg_type> {
                let mut resp = Option::<$msg_type>::None;
                self.send_request(
                    msg,
                    netlink_packet_core::NLM_F_REQUEST | netlink_packet_core::NLM_F_ACK,
                    |msg| {
                        if resp.is_some() {
                            return Err(anyhow::anyhow!(
                                "received multiple response messages to query request: {msg:?}"
                            ));
                        }
                        resp = Some(msg);
                        Ok(())
                    },
                )?;

                resp.ok_or(anyhow::anyhow!(
                    "received no response message to query request"
                ))
            }

            pub fn send_dump_request(&self, msg: $msg_type) -> anyhow::Result<Vec<$msg_type>> {
                let mut resps = Vec::<$msg_type>::new();
                self.send_request(
                    msg,
                    netlink_packet_core::NLM_F_REQUEST | netlink_packet_core::NLM_F_DUMP,
                    |msg| {
                        resps.push(msg);
                        Ok(())
                    },
                )?;
                Ok(resps)
            }
        }
    };
}

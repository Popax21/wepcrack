use netlink_packet_route::RouteNetlinkMessage;
use netlink_sys::protocols::NETLINK_ROUTE;

use crate::{netlink::NetlinkConnection, netlink_req_funcs};

pub struct RTNetlinkConnection(NetlinkConnection);

impl RTNetlinkConnection {
    pub fn new() -> anyhow::Result<RTNetlinkConnection> {
        Ok(RTNetlinkConnection(NetlinkConnection::new(NETLINK_ROUTE)?))
    }

    fn send_request(
        &self,
        msg: RouteNetlinkMessage,
        header_flags: u16,
        resp_cb: impl FnMut(RouteNetlinkMessage) -> anyhow::Result<()>,
    ) -> anyhow::Result<()> {
        self.0.send_request(msg, header_flags, resp_cb)
    }
}

netlink_req_funcs!(RTNetlinkConnection, RouteNetlinkMessage);

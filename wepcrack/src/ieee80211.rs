use std::{io::Read, rc::Rc, time::Duration};

use anyhow::Context;
use libc::{sockaddr_ll, sockaddr_storage, AF_PACKET, ETH_P_ALL, SOCK_RAW};
use netlink_packet_route::{
    link::{LinkFlag, LinkLayerType, LinkMessage},
    AddressFamily, RouteNetlinkMessage,
};
use radiotap::Radiotap;
use socket2::{Domain, SockAddr, Socket, Type};

use crate::{
    nl80211::{
        NL80211Channel, NL80211Connection, NL80211Interface, NL80211InterfaceType,
        NL80211RegulatoryDomain, NL80211Wiphy,
    },
    rtnetlink::RTNetlinkConnection,
    util::DropGuard,
};

pub struct IEEE80211Monitor {
    nl802111_con: Rc<NL80211Connection>,

    wiphy: NL80211Wiphy,
    channels: Vec<NL80211Channel>,

    orig_interfaces: Vec<NL80211Interface>,
    mon_interface: NL80211Interface,
}

impl IEEE80211Monitor {
    pub fn enter_monitor_mode(
        nl80211_con: Rc<NL80211Connection>,
        wiphy: NL80211Wiphy,
    ) -> anyhow::Result<IEEE80211Monitor> {
        //Obtain a list of all interfaces
        let orig_interfaces = NL80211Interface::query_list(&nl80211_con)
            .context("failed to query list of nl80211 interfaces")?
            .into_iter()
            .filter(|interf| interf.wiphy() == wiphy.index())
            .collect::<Vec<_>>();

        //Create a monitor interface
        let mon_interface = NL80211Interface::create_new(
            &nl80211_con,
            &wiphy,
            &(wiphy.name().to_owned() + "mon"),
            NL80211InterfaceType::Monitor,
            true,
        )
        .context("failed to create nl80211 monitor interface")?;

        let mut mon_guard = DropGuard::new(|| {
            _ = mon_interface.delete(&nl80211_con);
        });

        //Delete the original interfaces
        for iface in &orig_interfaces {
            iface
                .delete(&nl80211_con)
                .with_context(|| format!("failed to delete old nl80211 interface: {iface:?}"))?;
        }

        let mut orig_iface_guard = DropGuard::new(|| {
            for orig_if in &orig_interfaces {
                _ = NL80211Interface::create_new(
                    &nl80211_con,
                    &wiphy,
                    orig_if.name(),
                    orig_if.interface_type(),
                    false,
                );
            }
        });

        //Put the monitor interface into the up state
        let rtnetlink_con =
            RTNetlinkConnection::new().context("failed to create rtnetlink connection")?;
        rtnetlink_con
            .send_acked_request(RouteNetlinkMessage::NewLink({
                let mut msg = LinkMessage::default();
                msg.header.interface_family = AddressFamily::Unspec;
                msg.header.link_layer_type = LinkLayerType::Netrom;
                msg.header.index = mon_interface.index();
                msg.header.flags = vec![LinkFlag::Up, LinkFlag::Running];
                msg
            }))
            .context("failed to put monitor interface into up state")?;

        //Obtain a list of all permitted channels
        let channels = NL80211RegulatoryDomain::query_for_wiphy(&nl80211_con, &wiphy)
            .context("failed to query nl80211 wiphy regulatory domain")?
            .get_permitted_channels()
            .collect();

        //Disarm drop guards
        mon_guard.disarm();
        orig_iface_guard.disarm();
        drop(mon_guard);
        drop(orig_iface_guard);

        Ok(IEEE80211Monitor {
            nl802111_con: nl80211_con,

            wiphy,
            channels,

            orig_interfaces,
            mon_interface,
        })
    }

    pub fn channels(&self) -> &[NL80211Channel] {
        &self.channels
    }

    pub fn set_channel(&self, channel: NL80211Channel) -> anyhow::Result<()> {
        self.mon_interface.set_channel(&channel, &self.nl802111_con)
    }

    pub fn create_sniffer(&self) -> anyhow::Result<IEEE80211PacketSniffer> {
        //Create and bind a packet capture socket
        let packet_socket = Socket::new(Domain::from(AF_PACKET), Type::from(SOCK_RAW), None)
            .context("failed to create AF_PACKET socket")?;

        let mut sockaddr: sockaddr_storage = unsafe { std::mem::zeroed() };

        unsafe {
            //Setup the bind address
            *std::mem::transmute::<_, &mut sockaddr_ll>(&mut sockaddr) = sockaddr_ll {
                sll_family: AF_PACKET as u16,
                sll_protocol: (ETH_P_ALL as u16).to_be(),
                sll_ifindex: self.mon_interface.index() as i32,
                sll_hatype: 0,
                sll_pkttype: 0,
                sll_halen: 0,
                sll_addr: [0u8; 8],
            };
        }

        packet_socket
            .bind(&unsafe { SockAddr::new(sockaddr, std::mem::size_of::<sockaddr_ll>() as u32) })
            .context("failed to bind the PF_PACKET socket to the monitor interface")?;

        Ok(IEEE80211PacketSniffer(packet_socket))
    }
}

impl Drop for IEEE80211Monitor {
    fn drop(&mut self) {
        //Try to revert back the wiphy
        if let Err(err) = (|| -> anyhow::Result<()> {
            //Delete the monitor interface
            self.mon_interface.delete(&self.nl802111_con)?;

            //Create original interfaces again
            for orig_if in &self.orig_interfaces {
                NL80211Interface::create_new(
                    &self.nl802111_con,
                    &self.wiphy,
                    orig_if.name(),
                    orig_if.interface_type(),
                    false,
                )?;
            }

            Ok(())
        })() {
            eprintln!("failed to revert back wiphy after exiting monitor state: {err:?}")
        }
    }
}

pub struct IEEE80211PacketSniffer(Socket);

impl IEEE80211PacketSniffer {
    pub fn set_timeout(&mut self, timeout: Option<Duration>) -> anyhow::Result<()> {
        self.0
            .set_read_timeout(timeout)
            .context("failed to set 802.11 sniffer socket read timeout")?;
        self.0
            .set_write_timeout(timeout)
            .context("failed to set 802.11 sniffer socket write timeout")?;
        Ok(())
    }

    pub fn sniff_packet(&mut self) -> anyhow::Result<Option<IEEE80211Packet>> {
        //Receive a packet from the socket
        let mut rx_buf = [0u8; IEEE80211Packet::MAX_SIZE];

        let rx_size = 'rx_loop: loop {
            match self.0.read(&mut rx_buf) {
                Ok(rx_size) => break 'rx_loop rx_size,
                Err(err) if err.raw_os_error() == Some(11) => {
                    //Resource temporarily unavailable
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                Err(err) if err.kind() == std::io::ErrorKind::TimedOut => return Ok(None),
                Err(err) => {
                    return Err(
                        anyhow::anyhow!(err).context("failed to read packet from packet socket")
                    )
                }
            }
        };

        Ok(Some(
            IEEE80211Packet::try_from(&rx_buf[..rx_size])
                .context("failed to parse 802.11 packet")?,
        ))
    }

    pub fn inject_frame(&mut self, frame: &impl ieee80211::FrameTrait) -> anyhow::Result<()> {
        const IEEE80211_RADIOTAP_TX_FLAGS: u32 = 15;
        const IEEE80211_RADIOTAP_F_TX_NOACK: u16 = 0x8;

        //Send the packet through the socket
        let mut tx_buf = [0u8; IEEE80211Packet::MAX_SIZE];
        let tx_len = 10 + frame.bytes().len();
        tx_buf[2..4].copy_from_slice(&10u16.to_le_bytes());
        tx_buf[4..8].copy_from_slice(&((1 << IEEE80211_RADIOTAP_TX_FLAGS) as u32).to_le_bytes());
        tx_buf[8..10].copy_from_slice(&IEEE80211_RADIOTAP_F_TX_NOACK.to_le_bytes());

        tx_buf[10..tx_len].copy_from_slice(frame.bytes());

        let tx_size = 'tx_loop: loop {
            match self.0.send(&tx_buf[..tx_len]) {
                Ok(tx_size) => break 'tx_loop tx_size,
                Err(err) if err.raw_os_error() == Some(11) => {
                    //Resource temporarily unavailable
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
                Err(err) => {
                    return Err(
                        anyhow::anyhow!(err).context("failed to send packet through packet socket")
                    );
                }
            };
        };
        assert_eq!(tx_size, tx_len);

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct IEEE80211Packet {
    radiotap: Radiotap,
    data: Vec<u8>,
}

impl TryFrom<&[u8]> for IEEE80211Packet {
    type Error = anyhow::Error;

    fn try_from(buffer: &[u8]) -> Result<Self, Self::Error> {
        let (radiotap, data) = Radiotap::parse(buffer)?;

        Ok(IEEE80211Packet {
            radiotap,
            data: Vec::from(data),
        })
    }
}

impl IEEE80211Packet {
    pub const MAX_SIZE: usize = 16384;

    pub const fn radiotap(&self) -> &Radiotap {
        &self.radiotap
    }

    pub fn ieee80211_frame(&self) -> ieee80211::Frame {
        ieee80211::Frame::new(&self.data)
    }
}

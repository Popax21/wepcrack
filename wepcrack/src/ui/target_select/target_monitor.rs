use std::{
    collections::HashMap,
    rc::Rc,
    sync::{atomic::AtomicBool, Arc},
    thread::JoinHandle,
};

use ieee80211::{
    DSStatus, FrameLayer, FrameTrait, MacAddress, ManagementFrameLayer, ManagementFrameTrait,
    TaggedParametersTrait,
};

use crate::{
    ieee80211::{IEEE80211Monitor, IEEE80211PacketSniffer},
    nl80211::NL80211Channel,
    util::RecessiveMutex,
};

#[derive(Debug, Clone)]
pub struct TargetAccessPoint {
    mac_address: MacAddress,
    strength_dbm: f32,
    ssid: Option<String>,
}

impl TargetAccessPoint {
    pub const fn mac_address(&self) -> &MacAddress {
        &self.mac_address
    }

    pub const fn strength_dbm(&self) -> i32 {
        self.strength_dbm as i32
    }

    pub fn ssid(&self) -> Option<&str> {
        self.ssid.as_deref()
    }

    fn update_strength(&mut self, new_strength: i32) {
        const STRENGTH_BLEED: f32 = 0.9;

        self.strength_dbm =
            self.strength_dbm * STRENGTH_BLEED + new_strength as f32 * (1. - STRENGTH_BLEED);
    }
}

#[derive(Debug, Clone)]
pub struct TargetDevice {
    mac_address: MacAddress,
    strength_dbm: f32,
}

impl TargetDevice {
    pub const fn mac_address(&self) -> &MacAddress {
        &self.mac_address
    }

    pub const fn strength_dbm(&self) -> i32 {
        self.strength_dbm as i32
    }

    fn update_strength(&mut self, new_strength: i32) {
        const STRENGTH_BLEED: f32 = 0.9;

        self.strength_dbm =
            self.strength_dbm * STRENGTH_BLEED + new_strength as f32 * (1. - STRENGTH_BLEED);
    }
}

pub struct TargetMonitor {
    monitor: Rc<IEEE80211Monitor>,
    active_channel: Option<NL80211Channel>,

    should_exit: Arc<AtomicBool>,
    sniffer_thread: Option<JoinHandle<()>>,
    sniffer_thread_data: Arc<RecessiveMutex<SnifferThreadData>>,
}

impl TargetMonitor {
    pub fn new(monitor: Rc<IEEE80211Monitor>) -> Self {
        //Create the common sniffer thread data struct
        let sniffer_thread_data = SnifferThreadData {
            mode: TargetSnifferMode::Idle,
        };
        let sniffer_thread_data = Arc::new(RecessiveMutex::new(sniffer_thread_data));

        //Start the sniffer thread
        let should_exit = Arc::new(AtomicBool::new(false));
        let sniffer_thread = {
            let ieee80211_sniffer = monitor
                .create_sniffer()
                .expect("failed to create 802.11 sniffer for target monitor sniffer thread");

            let should_exit = should_exit.clone();
            let sniffer_thread_data = sniffer_thread_data.clone();

            std::thread::spawn(move || {
                sniffer_thread_func(
                    ieee80211_sniffer,
                    should_exit.as_ref(),
                    sniffer_thread_data.as_ref(),
                )
            })
        };

        TargetMonitor {
            monitor,
            active_channel: None,

            should_exit,
            sniffer_thread: Some(sniffer_thread),
            sniffer_thread_data,
        }
    }

    pub fn monitor(&self) -> &IEEE80211Monitor {
        self.monitor.as_ref()
    }

    pub fn active_channel(&self) -> Option<&NL80211Channel> {
        self.active_channel.as_ref()
    }

    pub fn set_channel(&mut self, channel: NL80211Channel) -> anyhow::Result<()> {
        self.monitor.set_channel(channel)?;
        self.active_channel = Some(channel);
        Ok(())
    }

    pub fn did_crash(&self) -> bool {
        !self.should_exit.load(std::sync::atomic::Ordering::SeqCst)
            && match self.sniffer_thread.as_ref() {
                Some(thread) => thread.is_finished(),
                None => true,
            }
    }

    pub fn sniff_aps(&mut self) {
        if let Ok(mut sniffer_data) = self.sniffer_thread_data.lock_dominant() {
            sniffer_data.mode = TargetSnifferMode::AccessPoints {
                access_points: HashMap::new(),
            };
        }
    }

    pub fn get_sniffed_aps(&self) -> Vec<TargetAccessPoint> {
        if let Ok(sniffer_data) = self.sniffer_thread_data.lock_dominant() {
            let TargetSnifferMode::AccessPoints { access_points } = &sniffer_data.mode else {
                panic!("target sniffer not currently sniffing for access points");
            };
            access_points.values().cloned().collect::<Vec<_>>()
        } else {
            Vec::default()
        }
    }

    pub fn sniff_devices(&mut self, ap_mac: MacAddress) {
        assert!(ap_mac.is_unicast());

        if let Ok(mut sniffer_data) = self.sniffer_thread_data.lock_dominant() {
            sniffer_data.mode = TargetSnifferMode::Devices {
                ap_mac,
                devices: HashMap::new(),
            };
        }
    }

    pub fn get_sniffed_devices(&self) -> Vec<TargetDevice> {
        if let Ok(sniffer_data) = self.sniffer_thread_data.lock_dominant() {
            let TargetSnifferMode::Devices { ap_mac: _, devices } = &sniffer_data.mode else {
                panic!("target sniffer not currently sniffing for target devices");
            };
            devices.values().cloned().collect::<Vec<_>>()
        } else {
            Vec::default()
        }
    }
}

impl Drop for TargetMonitor {
    fn drop(&mut self) {
        //Signal to the sniffer thread that it should exit
        self.should_exit
            .store(true, std::sync::atomic::Ordering::SeqCst);

        //Wait for the sniffer thread to exit
        if let Err(panic_res) = self.sniffer_thread.take().unwrap().join() {
            std::panic::resume_unwind(panic_res);
        }
    }
}

enum TargetSnifferMode {
    Idle,
    AccessPoints {
        access_points: HashMap<MacAddress, TargetAccessPoint>,
    },
    Devices {
        ap_mac: MacAddress,
        devices: HashMap<MacAddress, TargetDevice>,
    },
}

pub struct SnifferThreadData {
    mode: TargetSnifferMode,
}

fn sniff_ap_packet(
    sniffer: &mut IEEE80211PacketSniffer,
    access_points: &mut HashMap<MacAddress, TargetAccessPoint>,
) {
    //Sniff a packet
    let Some(packet) = sniffer
        .sniff_packet()
        .expect("failed to sniff a 802.11 packet")
    else {
        return;
    };
    let frame = packet.ieee80211_frame();
    let signal_strength_dbm = packet.radiotap().antenna_signal.map_or(0, |v| v.value) as i32;

    //Check if the frame is a beacon frame
    if let Some(FrameLayer::Management(management_frame)) = frame.next_layer() {
        if let Some(ManagementFrameLayer::Beacon(beacon)) = management_frame.next_layer() {
            //Register / Update the AP
            let Some(ap_mac) = beacon.transmitter_address() else {
                return;
            };
            if !ap_mac.is_unicast() {
                return;
            }

            let mut ssid: Option<String> =
                beacon.ssid().and_then(|ssid| String::from_utf8(ssid).ok());
            if ssid.as_ref().map_or(false, |ssid| ssid.is_empty()) {
                ssid = None;
            }

            match access_points.get_mut(&ap_mac) {
                Some(ap) => {
                    ap.update_strength(signal_strength_dbm);
                    if ssid.is_some() {
                        ap.ssid = ssid;
                    }
                }
                None => {
                    access_points.insert(
                        ap_mac,
                        TargetAccessPoint {
                            mac_address: ap_mac,
                            strength_dbm: signal_strength_dbm as f32,
                            ssid,
                        },
                    );
                }
            }

            return;
        }
    }

    //Extract the AP MAC address (if any)
    let ap_mac = match frame.ds_status() {
        DSStatus::NotLeavingDSOrADHOC => return,
        DSStatus::FromDSToSTA => MacAddress::from_bytes(&frame.bytes()[10..16]).unwrap(),
        DSStatus::FromSTAToDS => MacAddress::from_bytes(&frame.bytes()[4..10]).unwrap(),
        DSStatus::WDSOrMesh => return,
    };
    if !ap_mac.is_unicast() {
        return;
    }

    //Register / Update the AP
    match access_points.get_mut(&ap_mac) {
        Some(ap) => ap.update_strength(signal_strength_dbm),
        None => {
            access_points.insert(
                ap_mac,
                TargetAccessPoint {
                    mac_address: ap_mac,
                    strength_dbm: signal_strength_dbm as f32,
                    ssid: None,
                },
            );
        }
    }
}

fn sniff_dev_packet(
    sniffer: &mut IEEE80211PacketSniffer,
    target_ap_mac: &MacAddress,
    devices: &mut HashMap<MacAddress, TargetDevice>,
) {
    //Sniff a packet
    let Some(packet) = sniffer
        .sniff_packet()
        .expect("failed to sniff a 802.11 packet")
    else {
        return;
    };
    let frame = packet.ieee80211_frame();
    let signal_strength_dbm = packet.radiotap().antenna_signal.map_or(0, |v| v.value) as i32;

    //Extract and check the AP MAC address (if any)
    let ap_mac = match frame.ds_status() {
        DSStatus::NotLeavingDSOrADHOC => return,
        DSStatus::FromDSToSTA => MacAddress::from_bytes(&frame.bytes()[10..16]).unwrap(),
        DSStatus::FromSTAToDS => MacAddress::from_bytes(&frame.bytes()[4..10]).unwrap(),
        DSStatus::WDSOrMesh => return,
    };
    if &ap_mac != target_ap_mac {
        return;
    }

    //Extract the device MAC address
    let dev_mac = match frame.ds_status() {
        DSStatus::NotLeavingDSOrADHOC => return,
        DSStatus::FromDSToSTA => MacAddress::from_bytes(&frame.bytes()[4..10]).unwrap(),
        DSStatus::FromSTAToDS => MacAddress::from_bytes(&frame.bytes()[10..16]).unwrap(),
        DSStatus::WDSOrMesh => return,
    };
    if !dev_mac.is_unicast() {
        return;
    }

    //Register / Update the AP
    match devices.get_mut(&dev_mac) {
        Some(dev) => dev.update_strength(signal_strength_dbm),
        None => {
            devices.insert(
                dev_mac,
                TargetDevice {
                    mac_address: dev_mac,
                    strength_dbm: signal_strength_dbm as f32,
                },
            );
        }
    }
}

fn sniffer_thread_func(
    mut sniffer: IEEE80211PacketSniffer,
    should_exit: &AtomicBool,
    data: &RecessiveMutex<SnifferThreadData>,
) {
    sniffer
        .set_timeout(Some(std::time::Duration::from_secs(1)))
        .expect("failed to set 802.11 sniffer timeout");

    while !should_exit.load(std::sync::atomic::Ordering::SeqCst) {
        //Lock the sniffer thread data
        let Ok(mut data) = data.lock_recessive() else {
            return;
        };

        //Execute the requested logic
        match &mut data.mode {
            TargetSnifferMode::Idle => std::thread::yield_now(),
            TargetSnifferMode::AccessPoints { access_points } => {
                sniff_ap_packet(&mut sniffer, access_points)
            }
            TargetSnifferMode::Devices { ap_mac, devices } => {
                sniff_dev_packet(&mut sniffer, &ap_mac, devices)
            }
        }
    }
}

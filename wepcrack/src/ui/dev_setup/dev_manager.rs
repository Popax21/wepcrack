use std::error::Error;

use crate::nl80211::{NL80211Connection, NL80211Interface, NL80211InterfaceType, NL80211Wiphy};

pub(super) struct Device {
    wiphy: NL80211Wiphy,
    interfaces: Vec<NL80211Interface>,
    supports_monitor_mode: bool,
}

impl Device {
    fn from_wiphy(wiphy: NL80211Wiphy) -> Device {
        Device {
            interfaces: Vec::default(),

            supports_monitor_mode: wiphy
                .supported_interface_types()
                .contains(&NL80211InterfaceType::Monitor),

            wiphy,
        }
    }

    pub fn name(&self) -> &str {
        self.wiphy.name()
    }

    pub fn interfaces(&self) -> &[NL80211Interface] {
        &self.interfaces
    }

    pub fn supports_monitor_mode(&self) -> bool {
        self.supports_monitor_mode
    }

    pub fn is_suitable(&self) -> bool {
        self.supports_monitor_mode
    }
}

pub(super) struct DevManager {
    nl82011_con: NL80211Connection,
    devices: Vec<Device>,
}

impl DevManager {
    pub fn new() -> Result<DevManager, Box<dyn Error>> {
        //Create a new nl80211 connection
        let nl82011_con: NL80211Connection = NL80211Connection::new()?;

        //Obtain a list of all nl80211 wiphys and interfaces
        let wiphys = NL80211Wiphy::query_list(&nl82011_con)?;
        let interfaces = NL80211Interface::query_list(&nl82011_con)?;

        //Create a list of all devices
        let mut devices = wiphys
            .into_iter()
            .map(Device::from_wiphy)
            .collect::<Vec<_>>();

        for interf in interfaces.into_iter() {
            devices[interf.wiphy() as usize].interfaces.push(interf);
        }

        Ok(DevManager {
            nl82011_con,
            devices,
        })
    }

    pub fn devices(&self) -> &[Device] {
        &self.devices
    }
}

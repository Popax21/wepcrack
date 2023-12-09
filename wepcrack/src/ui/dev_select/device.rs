use std::{error::Error, path::PathBuf};

use anyhow::Context;

use crate::nl80211::{NL80211Connection, NL80211Interface, NL80211InterfaceType, NL80211Wiphy};

pub(super) struct Device {
    wiphy: NL80211Wiphy,
    interfaces: Vec<NL80211Interface>,
    rfkill: Option<DeviceRFKill>,
    supports_monitor_mode: bool,
}

impl Device {
    fn from_wiphy(wiphy: NL80211Wiphy) -> Device {
        //Try to obtain the rfkill of the wiphy
        let rfkill_path = 'rfkill_find: {
            let mut wiphy_path = PathBuf::from("/sys/class/ieee80211");
            wiphy_path.push(wiphy.name());

            for entry in
                std::fs::read_dir(wiphy_path).expect("failed to read wiphy sysfs directory")
            {
                let entry = entry.expect("failed to read wiphy sysfs directory");
                if entry.file_name().to_str().unwrap().starts_with("rfkill") {
                    break 'rfkill_find Some(entry.path());
                }
            }
            None
        };

        //Check if the device supports monitor mode
        let supports_monitor_mode = wiphy
            .supported_interface_types()
            .contains(&NL80211InterfaceType::Monitor);

        Device {
            wiphy,
            interfaces: Vec::default(), //This gets populated later
            rfkill: rfkill_path.map(DeviceRFKill::from_path),
            supports_monitor_mode,
        }
    }

    pub fn name(&self) -> &str {
        self.wiphy.name()
    }

    pub const fn wiphy(&self) -> &NL80211Wiphy {
        &self.wiphy
    }

    pub fn interfaces(&self) -> &[NL80211Interface] {
        &self.interfaces
    }

    pub fn rfkill(&self) -> Option<&DeviceRFKill> {
        self.rfkill.as_ref()
    }

    pub fn supports_monitor_mode(&self) -> bool {
        self.supports_monitor_mode
    }

    pub fn is_suitable(&self) -> bool {
        self.supports_monitor_mode
    }
}

pub(super) struct DeviceRFKill {
    path: PathBuf,
    name: String,
}

impl DeviceRFKill {
    fn from_path(path: PathBuf) -> DeviceRFKill {
        DeviceRFKill {
            name: path.file_name().unwrap().to_str().unwrap().to_string(),
            path,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn is_soft_locked(&self) -> bool {
        let mut path = self.path.clone();
        path.push("soft");
        std::fs::read_to_string(path)
            .expect("failed to read rfkill soft kill state")
            .trim()
            .parse::<i32>()
            .unwrap()
            != 0
    }

    pub fn is_hard_locked(&self) -> bool {
        let mut path = self.path.clone();
        path.push("hard");
        std::fs::read_to_string(path)
            .expect("failed to read rfkill hard kill state")
            .trim()
            .parse::<i32>()
            .unwrap()
            != 0
    }
}

pub(super) struct DeviceList(Vec<Device>);

impl DeviceList {
    pub fn query_list(nl80211_con: &NL80211Connection) -> Result<DeviceList, Box<dyn Error>> {
        //Obtain a list of all nl80211 wiphys and interfaces
        let wiphys =
            NL80211Wiphy::query_list(nl80211_con).context("failed to query nl80211 wiphy list")?;
        let interfaces = NL80211Interface::query_list(nl80211_con)
            .context("failed to query nl80211 device list")?;

        //Create a list of all devices
        let mut devices = wiphys
            .into_iter()
            .map(Device::from_wiphy)
            .collect::<Vec<_>>();

        for interf in interfaces.into_iter() {
            devices[interf.wiphy() as usize].interfaces.push(interf);
        }

        Ok(DeviceList(devices))
    }

    pub fn devices(&self) -> &[Device] {
        &self.0
    }
}

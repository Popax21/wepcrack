use anyhow::Context;

use crate::nl80211::{NL80211Connection, NL80211Interface, NL80211InterfaceType, NL80211Wiphy};

pub struct IEEE80211Monitor {
    nl802111_con: NL80211Connection,
    wiphy: NL80211Wiphy,
    orig_interfaces: Vec<NL80211Interface>,
    mon_interface: NL80211Interface,
}

impl IEEE80211Monitor {
    pub fn enter_monitor_mode(
        nl802111_con: NL80211Connection,
        wiphy: NL80211Wiphy,
    ) -> anyhow::Result<IEEE80211Monitor> {
        //Obtain a list of all interfaces
        let orig_interfaces = NL80211Interface::query_list(&nl802111_con)
            .context("failed to query list of nl80211 interfaces")?
            .into_iter()
            .filter(|interf| interf.wiphy() == wiphy.index())
            .collect::<Vec<_>>();

        //Create a monitor interface
        let mon_interface = NL80211Interface::create_new(
            &nl802111_con,
            &wiphy,
            &(wiphy.name().to_owned() + "mon"),
            NL80211InterfaceType::Monitor,
        )
        .context("failed to create nl80211 monitor interface")?;

        //Delete the original interfaces
        for iface in &orig_interfaces {
            iface
                .delete(&nl802111_con)
                .with_context(|| format!("failed to delete old nl80211 interface: {iface:?}"))?;
        }

        Ok(IEEE80211Monitor {
            nl802111_con,
            wiphy,
            orig_interfaces,
            mon_interface,
        })
    }

    pub const fn wiphy(&self) -> &NL80211Wiphy {
        &self.wiphy
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
                )?;
            }

            Ok(())
        })() {
            eprintln!("failed to revert back wiphy after exiting monitor state: {err:?}")
        }
    }
}

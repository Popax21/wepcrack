use std::rc::Rc;

use crate::{ieee80211::IEEE80211Monitor, nl80211::NL80211Channel};

pub struct TargetMonitor {
    monitor: Rc<IEEE80211Monitor>,
    active_channel: Option<NL80211Channel>,
}

impl TargetMonitor {
    pub fn new(monitor: Rc<IEEE80211Monitor>) -> Self {
        TargetMonitor {
            monitor,
            active_channel: None,
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
}

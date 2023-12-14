use std::rc::Rc;

use crate::ieee80211::IEEE80211Monitor;

pub struct TargetMonitor {
    monitor: Rc<IEEE80211Monitor>,
}

impl TargetMonitor {
    pub fn new(monitor: Rc<IEEE80211Monitor>) -> Self {
        TargetMonitor { monitor }
    }

    pub fn monitor(&self) -> &IEEE80211Monitor {
        self.monitor.as_ref()
    }
}

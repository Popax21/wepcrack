use std::error::Error;

use crate::nl80211::{NL80211Connection, NL80211Whipy};

pub(super) struct DevManager {
    nl82011_con: NL80211Connection,
    whipys: Vec<NL80211Whipy>,
}

impl DevManager {
    pub fn new() -> Result<DevManager, Box<dyn Error>> {
        //Create a new nl80211 connection
        let nl82011_con: NL80211Connection = NL80211Connection::new()?;

        //Obtain a list of all nl80211 wiphys
        let whipys = NL80211Whipy::query_list(&nl82011_con)?;

        Ok(DevManager {
            nl82011_con,
            whipys,
        })
    }

    pub fn whipys(&self) -> &Vec<NL80211Whipy> {
        &self.whipys
    }
}

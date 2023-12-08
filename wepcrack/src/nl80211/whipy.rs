use std::error::Error;

use super::{
    NL80211Attribute, NL80211AttributeTag, NL80211Command, NL80211Connection, NL80211Message,
};

#[derive(Debug, Clone)]
pub struct NL80211Whipy {
    name: String,
}

impl NL80211Whipy {
    pub fn query_list(con: &NL80211Connection) -> Result<Vec<NL80211Whipy>, Box<dyn Error>> {
        //Send a dump GET_WHIPY request
        con.send_request(
            NL80211Message {
                cmd: NL80211Command::GetWiphy,
                nlas: vec![],
            },
            true,
        )?;
        let whipys = con.recv_dump_response(NL80211Command::NewWiphy)?;

        //Create a clean list of all whipys
        let whipys = whipys
            .iter()
            .map(|msg| {
                //Parse the message attributes
                let NL80211Attribute::WhipyName(name) = msg
                    .find_attr(NL80211AttributeTag::WhipyName)
                    .expect("whipy has no name attribute")
                else {
                    unreachable!()
                };

                //Create a new whipy
                NL80211Whipy { name: name.clone() }
            })
            .collect();

        Ok(whipys)
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

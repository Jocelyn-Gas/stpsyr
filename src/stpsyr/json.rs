use crate::Stpsyr;
extern crate serde_json;

impl Stpsyr {
    pub fn dump_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}
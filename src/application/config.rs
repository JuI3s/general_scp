use std::{
    fs,
    net::{Ipv4Addr, SocketAddrV4},
    process::exit,
};

use serde::{Deserialize, Serialize};

use super::quorum::{QuorumSet, QuorumSlice};

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub path: String,
    pub quorum_set: QuorumSet,
}

impl Config {
    pub fn new_test_config() -> Self {
        todo!();
        // let ip_addr = Ipv4Addr::new(127, 0, 0, 1);
        // let sock1 = SocketAddrV4::new(ip_addr, 17);
        // let sock2 = SocketAddrV4::new(ip_addr, 18);

        // let quorum_set1 = QuorumSlice::from([sock1.clone(), sock2.clone()]);
        // let quorum_set2 = QuorumSlice::from([sock1.clone()]);
        // let quorum_set = QuorumSet::from([quorum_set1, quorum_set2]);

        // Config {
        //     path: format!("new"),
        //     quorum_set: quorum_set,
        // }
    }

    pub fn to_toml_string(&self) -> String {
        toml::to_string(self).unwrap()
    }

    pub fn from_toml_file(filename: &String) -> Self {
        let contents = match fs::read_to_string(filename) {
            Ok(c) => c,
            Err(_) => {
                eprintln!("Could not read config file `{}`", filename);
                exit(1);
            }
        };

        let config: Config = match toml::from_str(&contents) {
            Ok(d) => d,
            Err(_) => {
                eprintln!("Unable to load config data from `{}`", filename);
                exit(1);
            }
        };
        config
    }
}

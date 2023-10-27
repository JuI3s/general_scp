use std::{fs, process::exit};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub path: String,
    pub quorum: Vec<Vec<String>>,
}

impl Config {
    pub fn new() -> Self {
        // TODO: remove below for testing.
        let mut quorum = Vec::new();
        let mut quorum_set1 = Vec::new();
        let mut quorum_set2 = Vec::new();
        quorum_set1.push(format!("127.0.0.1:13"));
        quorum_set1.push(format!("127.0.0.1:1"));
        quorum_set2.push(format!("124.123.1.51:5"));
        quorum.push(quorum_set1);
        quorum.push(quorum_set2);

        Config {
            path: format!("new"),
            quorum: quorum,
        }
    }

    pub fn from_toml_file(filename: &String) -> Self {
        let contents = match fs::read_to_string(filename) {
            // If successful return the files text as `contents`.
            // `c` is a local variable.
            Ok(c) => c,
            // Handle the `error` case.
            Err(_) => {
                // Write `msg` to `stderr`.
                eprintln!("Could not read config file `{}`", filename);
                // Exit the program with exit code `1`.
                exit(1);
            }
        };

        // Use a `match` block to return the
        // file `contents` as a `Data struct: Ok(d)`
        // or handle any `errors: Err(_)`.
        let config: Config = match toml::from_str(&contents) {
            // If successful, return data as `Data` struct.
            // `d` is a local variable.
            Ok(d) => d,
            // Handle the `error` case.
            Err(_) => {
                // Write `msg` to `stderr`.
                eprintln!("Unable to load config data from `{}`", filename);
                // Exit the program with exit code `1`.
                exit(1);
            }
        };
        config
    }
}

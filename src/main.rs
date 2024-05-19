use std::fs::File;

use anyhow::Result;
use chrono::{DateTime, Local};
//use smol::{prelude::*};

use crate::domain::ilet;
use crate::domain::nightscout;
use crate::domain::nightscout::iLet_to_ns_server;
use crate::models::ilet::iLetSecrets;
//use crate::models::nightscout;
use crate::models::nightscout_web::NightscoutSecrets;

mod domain;
mod models;

fn main() {

    let (ilet_config, nightscout_config) = match read_config() {
        Ok((ilet, ns)) => (ilet, ns),
        Err(error) => panic!("Could not get config: {:?}", error),
    };

    let client = reqwest::blocking::Client::new();

    let ns_bearer = nightscout::generate_token(&nightscout_config, &client);

    let ilet_bearer = ilet::authenticate_iLet(&ilet_config, &client);

    //TODO! Accept external timestamps, there are DEFINITELY limits to time ranges before iLet servers return an error
    let ilet_data = ilet::grab_iLet_data(
        &client,
        ilet_bearer.unwrap(),
        ilet_config.serial_number,
        DateTime::from_timestamp(1710864250, 0)
            .unwrap()
            .with_timezone(&Local),
        DateTime::from_timestamp(1716073850, 0)
            .unwrap()
            .with_timezone(&Local),
    );

    iLet_to_ns_server(ilet_data.unwrap(), &client, ns_bearer.unwrap(), nightscout_config);

}

fn read_config() -> Result<(iLetSecrets, NightscoutSecrets)> {
    let config = File::open("config.json").expect("No config.json file found!");

    let val: serde_json::Value = serde_json::from_reader(config).expect("JSON malformed!");

    let iLet: iLetSecrets = serde_json::from_value(val["iLet"].clone()).expect("Could not read iLet config.");

    let ns: NightscoutSecrets = serde_json::from_value(val["Nightscout"].clone()).expect("Could not read Nightscout config.");

    Ok((iLet, ns))
}

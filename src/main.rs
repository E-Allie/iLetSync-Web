use chrono::{DateTime, Local};

use crate::domain::ilet;

mod domain;

fn main() {
    let client = reqwest::blocking::Client::new();

    let user_credentials = ilet::user_credentials();

    let serial_number = ilet::user_serial_number();

    let bearer_info = ilet::authenticate_iLet(user_credentials.unwrap(), &client);

    let final_data = ilet::grab_iLet_data(
        &client,
        bearer_info.unwrap(),
        serial_number.unwrap(),
        DateTime::from_timestamp(1710533974, 0)
            .unwrap()
            .with_timezone(&Local),
        DateTime::from_timestamp(1710743574, 0)
            .unwrap()
            .with_timezone(&Local),
    );

    println!("{:?}", final_data)
}

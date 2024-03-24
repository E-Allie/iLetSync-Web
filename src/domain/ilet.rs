use std::io;
use std::time::Duration;
use secrecy::{ExposeSecret, SecretString};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use serde_json::Result;

//Represents the exact JSON response from iLet
//Subject to change with more user reports, I am unsure of how some values are represented
//EX: I do not know what a HI value would appear as
//EX: I do not know how bg-run mode appears
#[derive(Serialize, Deserialize)]
struct iLetData {
    serial_number: String,
    app_adjusted_time: Duration,
    app_local_time: Duration,
    step_index: u64,        //This value MIGHT monotonically increase once every 5 minutes, forever
    cgm_value: i16,         //Seems to only take ints [-1, 400]?
    basal_insulin_delivered: f64,
    total_insulin_delivered: f64,
    glucagon_delivered: f64,
    body_mass: u64,
    meal_type: u8,
    meal_dose: u8,
    meal_size: u8,
    cf: u8,                 //I don't believe this can take decimal values
    bR1: f64,
    bR2: f64,
    bR3: f64,
    bR4: f64,
    mdi: f64,
    breakfast: f64,
    lunch: f64,
    dinner: f64,
    cgm_target: u8,
    bgm_value: i16
}

//iLet Servers expect authentication in the form "USERNAME:PASSWORD" base64 encoded
pub fn user_credentials() -> SecretString {

    let mut credentials_in = String::new();

    println!("Username/Email:");

    io::stdin().read_line(&mut credentials_in).unwrap();
    credentials_in.pop();
    credentials_in.push_str(":");

    println!("Password:");

    io::stdin().read_line(&mut credentials_in).unwrap();

    credentials_in.pop();

    SecretString::new(STANDARD.encode(credentials_in))

}

//Serial Number of the iLet Device being accessed
pub fn user_serial_number() -> String {

    let mut input = String::new();

    println!("iLet Serial Number [Ex: A123456]: ");

    io::stdin().read_line(&mut input).unwrap();

    input

}

//Request "bearer" token
pub fn authenticate_iLet(credentials: SecretString, client: reqwest::blocking::Client) -> SecretString {

    let mut auth_url = "https://us-users.betabionicsapi.com/2/account/auth?encodedAppID=".to_owned();

    let uuid= STANDARD.encode(Uuid::new_v4().hyphenated().to_string());

    auth_url.push_str(&uuid);

    let bearer = client
        .get(auth_url)
        .header("Authorization", "Basic ".to_owned() + credentials.expose_secret())
        .send();

    println!("{:?}",bearer.unwrap().text());
    
    SecretString::new("".to_string())

}

//https://us-apps.betabionicsapi.com/1/reporting/clinicalstepdata?serialNumber=A107311&epochStartDate=1710701695&epochEndDate=1711306495
pub fn grab_iLet_data() -> u8 {
    
    let uuid = Uuid::new_v4();

    1

}
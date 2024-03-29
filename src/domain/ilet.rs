use std::io;

use anyhow::{anyhow, Result};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use chrono::Local;
use reqwest::blocking::Client;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug)]
enum ServerErrors {
    BadUserInfo,
    ServerSideIssue,
}

impl std::fmt::Display for ServerErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadUserInfo => write!(f, "Check Inputted Credentials."),
            Self::ServerSideIssue => write!(f, "iLet Server gave an unexpected response, retry."),
        }
    }
}

impl std::error::Error for ServerErrors {}

#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct iLetAuthResponse {
    message: Option<String>,
    status_code: i16,
    access_token: Option<String>,
    id_token: String,
    refresh_token: String,
    session: Option<String>,
    user_name: Option<String>,
}

//Represents the exact JSON response from iLet
//Subject to change with more user reports, I am unsure of how some values are represented
//EX: I do not know what a HI value would appear as
//EX: I do not know how bg-run mode appears
#[allow(non_snake_case)]
#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct iLetData {
    serial_number: String,
    app_adjusted_time: String,  //TODO: Time type
    app_local_time: String,
    step_index: u64, //This value MIGHT monotonically increase once every 5 minutes, forever
    cgm_value: i16,  //Seems to only take ints [-1, 400]?
    basal_insulin_delivered: f64,
    total_insulin_delivered: f64,
    glucagon_delivered: f64,
    body_mass: u64,
    meal_type: u8,
    meal_dose: u8,
    meal_size: u8,
    cf: u8, //I don't believe this can take decimal values
    bR1: f64,
    bR2: f64,
    bR3: f64,
    bR4: f64,
    mdi: f64,
    breakfast: f64,
    lunch: f64,
    dinner: f64,
    cgm_target: u8,
    bgm_value: i16,
}

//iLet Servers expect authentication in the form "USERNAME:PASSWORD" base64 encoded
pub fn user_credentials() -> Result<SecretString> {
    let mut credentials_in = String::new();

    println!("Username/Email:");

    io::stdin().read_line(&mut credentials_in)?;
    credentials_in.pop();
    credentials_in.push_str(":");

    println!("Password:");

    io::stdin().read_line(&mut credentials_in)?;

    credentials_in.pop();

    Ok(SecretString::new(STANDARD.encode(credentials_in)))
}

//Serial Number of the iLet Device being accessed
pub fn user_serial_number() -> Result<String> {
    let mut input = String::new();

    println!("iLet Serial Number [Ex: A123456]: ");

    io::stdin().read_line(&mut input)?;

    input.pop();

    Ok(input)
}

//Request "bearer" token

//TODO: Utilize the refresh token for longer lifetime runs
pub fn authenticate_iLet(credentials: SecretString, client: &Client) -> Result<iLetAuthResponse> {
    let mut auth_url =
        "https://us-users.betabionicsapi.com/2/account/auth?encodedAppID=".to_owned();

    let uuid = STANDARD.encode(Uuid::new_v4().hyphenated().to_string());

    auth_url.push_str(&uuid);

    let bearer_req = client.get(auth_url).header(
        "Authorization",
        "Basic ".to_owned() + credentials.expose_secret(),
    );

    match bearer_req.send() {
        Ok(resp) => {
            if resp.status().is_success() {
                return resp.json().map_err(anyhow::Error::from); //TODO: Deal with json deserialize error. In practice, I have never see a differently-formatted JSON response from iLet
            } else if resp.status().is_server_error() {
                return Err(anyhow!(ServerErrors::ServerSideIssue));
            } else if resp.status().is_client_error() {
                return Err(anyhow!(ServerErrors::BadUserInfo));
            } else {
                println!("Something else happened. Status: {:?}", resp.status());
                return Err(anyhow!("Unexpected"));
            }
        }

        Err(e) => return Err(anyhow!("Initial Request Unexpectedly Failed! {:?}", e)),
    };
}

//TODO: Flesh out error messages and retry
pub fn grab_iLet_data(
    client: &Client,
    iLet_auth_data: iLetAuthResponse,
    serial_number: String,
    start: chrono::DateTime<Local>,
    end: chrono::DateTime<Local>,
) -> Result<Vec<iLetData>> {
    let report_url = "https://us-apps.betabionicsapi.com/1/reporting/clinicalstepdata?";

    let data_req = client
        .get(report_url)
        .query(&[
            ("serialNumber", serial_number),
            ("epochStartDate", start.timestamp().to_string()),
            ("epochEndDate", end.timestamp().to_string()),
        ])
        .header(
            "Authorization",
            "Bearer ".to_owned() + &iLet_auth_data.id_token,
        );

    match data_req.send() {
        Ok(resp) => {
            if resp.status().is_success() {
                return resp.json().map_err(anyhow::Error::from);
            } else if resp.status().is_server_error() {
                return Err(anyhow!(ServerErrors::ServerSideIssue));
            } else if resp.status().is_client_error() {
                return Err(anyhow!(ServerErrors::BadUserInfo));
            } else {
                println!("Something else happened. Status: {:?}", resp.status());
                return Err(anyhow!("Unexpected"));
            }
        }

        Err(e) => return Err(anyhow!("Initial Request Unexpectedly Failed! {:?}", e)),
    };
}

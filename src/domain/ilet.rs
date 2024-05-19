use std::time::Duration;
use anyhow::{anyhow, Result};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use chrono::Local;
use reqwest::blocking::Client;
use uuid::Uuid;

use crate::models::ilet::{iLetAuthResponse, iLetData, iLetSecrets, ServerErrors};

//Request "bearer" token
//TODO: Utilize the refresh token for longer lifetime runs
pub fn authenticate_iLet(iLet_conf: &iLetSecrets, client: &Client) -> Result<iLetAuthResponse> {

    //Servers expect user secrets as "USERNAME:PASSWORD" to base64

    let credentials = STANDARD.encode([iLet_conf.username.clone(),iLet_conf.password.clone()].join(":"));

    let mut auth_url =
        "https://us-users.betabionicsapi.com/2/account/auth?encodedAppID=".to_owned();

    let uuid = STANDARD.encode(Uuid::new_v4().hyphenated().to_string());

    auth_url.push_str(&uuid);

    let bearer_req = client.get(auth_url).header(
        "Authorization",
        "Basic ".to_owned() + &credentials,
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
        )
        .timeout(Duration::new(120,0));

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

        Err(e) => return Err(anyhow!("iLet Grab Initial Request Unexpectedly Failed! {:?}", e)),
    };
}

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;


#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct iLetSecrets {
    pub(crate) username: String,
    pub(crate) password: String,
    pub(crate) serial_number: String,
}

#[derive(Debug)]
pub(crate) enum ServerErrors {
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
#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct iLetAuthResponse {
    message: Option<String>,
    status_code: i16,
    access_token: Option<String>,
    pub(crate) id_token: String,
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
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct iLetData {
    serial_number: String,
    app_adjusted_time: String,  //TODO: Time type
    pub(crate) app_local_time: String,
    step_index: u64, //This value MIGHT monotonically increase once every 5 minutes, forever
    pub(crate) cgm_value: i16,  //Seems to only take ints [-1, 400]?
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub(crate) basal_insulin_delivered: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub(crate) total_insulin_delivered: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    glucagon_delivered: Decimal,
    body_mass: u64,
    pub(crate) meal_type: u8,
    pub(crate) meal_dose: u8,
    pub(crate) meal_size: u8,
    cf: u8, //I don't believe this can take decimal values
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    bR1: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    bR2: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    bR3: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    bR4: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    mdi: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    breakfast: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    lunch: Decimal,
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    dinner: Decimal,
    cgm_target: u8,
    pub(crate) bgm_value: i16,
}

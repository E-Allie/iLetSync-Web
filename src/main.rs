mod domain;
use domain::ilet::user_credentials;

use std::str::FromStr;
use crate::domain::ilet;
use crate::domain::ilet::authenticate_iLet;

fn main() {
    let client = reqwest::blocking::Client::new();

    authenticate_iLet(user_credentials(), client);

}

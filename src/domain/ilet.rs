//! Handles interactions with the iLet API.
//!
//! This module is responsible for authenticating with the iLet servers
//! and fetching clinical step data within a specified time range.

use std::time::Duration;
use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use chrono::{DateTime, Local};
use reqwest::blocking::Client;
use uuid::Uuid;

use crate::models::ilet::{iLetAuthResponse, iLetData, iLetSecrets}; // ServerErrors enum removed from here

// Custom error types for more specific iLet API feedback
#[derive(Debug, thiserror::Error)]
pub enum iLetApiError {
    #[error("iLet server error (5xx): {0}")]
    ServerSideIssue(String),
    #[error("iLet client error (4xx) - bad user info or request: {0}")]
    BadUserInfoOrRequest(String),
    #[error("iLet request timed out after {0:?} seconds.")]
    Timeout(u64),
    #[error("Unexpected response status from iLet server: {0} - {1}")]
    UnexpectedStatus(reqwest::StatusCode, String),
    #[error("Request to iLet server failed: {0}")]
    RequestFailed(#[from] reqwest::Error),
    #[error("JSON deserialization failed: {0}. Response body might not be as expected.")]
    DeserializationError(String),
}

const DEFAULT_TIMEOUT_SECONDS: u64 = 120;

/// Authenticates with the iLet server to obtain an API access token.
///
/// # Arguments
/// * `ilet_conf` - User's iLet credentials.
/// * `client` - A `reqwest::blocking::Client` for making HTTP requests.
///
/// # Returns
/// A `Result` containing the `iLetAuthResponse` (with tokens) if successful, or an `iLetApiError`.
pub fn authenticate_iLet(ilet_conf: &iLetSecrets, client: &Client) -> Result<iLetAuthResponse> {
    // iLet servers expect credentials as "USERNAME:PASSWORD" encoded in Base64.
    let credentials = STANDARD.encode(format!("{}:{}", ilet_conf.username, ilet_conf.password));
    
    // The encodedAppID seems to be a Base64 encoded UUID.
    let encoded_app_id = STANDARD.encode(Uuid::new_v4().hyphenated().to_string());
    let auth_url = format!(
        "https://us-users.betabionicsapi.com/2/account/auth?encodedAppID={}",
        encoded_app_id
    );

    let response = client
        .get(&auth_url)
        .header("Authorization", format!("Basic {}", credentials))
        .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECONDS))
        .send()
        .map_err(iLetApiError::RequestFailed)
        .context("iLet authentication request failed to send.")?;

    let status = response.status();
    if status.is_success() {
        response
            .json::<iLetAuthResponse>()
            .map_err(|e| iLetApiError::DeserializationError(e.to_string()))
            .context("Failed to deserialize iLet authentication response.")
    } else if status.is_server_error() {
        let text = response.text().unwrap_or_else(|_| "Failed to read error response body".to_string());
        Err(anyhow!(iLetApiError::ServerSideIssue(text)))
    } else if status.is_client_error() {
        let text = response.text().unwrap_or_else(|_| "Failed to read error response body".to_string());
        Err(anyhow!(iLetApiError::BadUserInfoOrRequest(text)))
    } else {
        let text = response.text().unwrap_or_else(|_| "Failed to read error response body".to_string());
        Err(anyhow!(iLetApiError::UnexpectedStatus(status, text)))
    }
}

/// Fetches clinical step data from the iLet server for a given user and time range.
///
/// # Arguments
/// * `client` - A `reqwest::blocking::Client`.
/// * `ilet_auth_data` - Authentication tokens obtained from `authenticate_iLet`.
/// * `serial_number` - The serial number of the iLet device.
/// * `start` - The start of the date range (`DateTime<Local>`).
/// * `end` - The end of the date range (`DateTime<Local>`).
///
/// # Returns
/// A `Result` containing a vector of `iLetData` points if successful, or an `iLetApiError`.
pub fn grab_iLet_data(
    client: &Client,
    ilet_auth_data: iLetAuthResponse, // Taking ownership as id_token is consumed
    serial_number: String,        // Taking ownership
    start: DateTime<Local>,
    end: DateTime<Local>,
) -> Result<Vec<iLetData>> {
    let report_url = "https://us-apps.betabionicsapi.com/1/reporting/clinicalstepdata";

    let response = client
        .get(report_url)
        .query(&[
            ("serialNumber", serial_number),
            ("epochStartDate", start.timestamp().to_string()),
            ("epochEndDate", end.timestamp().to_string()),
        ])
        .header("Authorization", format!("Bearer {}", ilet_auth_data.id_token))
        .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECONDS))
        .send()
        .map_err(|e| { // Check for timeout specifically
            if e.is_timeout() {
                iLetApiError::Timeout(DEFAULT_TIMEOUT_SECONDS)
            } else {
                iLetApiError::RequestFailed(e)
            }
        })
        .context("iLet data grab request failed to send.")?;

    let status = response.status();
    if status.is_success() {
        response
            .json::<Vec<iLetData>>()
            .map_err(|e| iLetApiError::DeserializationError(e.to_string()))
            .context("Failed to deserialize iLet data response.")
    } else if status.is_server_error() {
        let text = response.text().unwrap_or_else(|_| "Failed to read error response body".to_string());
        Err(anyhow!(iLetApiError::ServerSideIssue(text)))
    } else if status.is_client_error() {
        let text = response.text().unwrap_or_else(|_| "Failed to read error response body".to_string());
        Err(anyhow!(iLetApiError::BadUserInfoOrRequest(text)))
    } else {
        let text = response.text().unwrap_or_else(|_| "Failed to read error response body".to_string());
        Err(anyhow!(iLetApiError::UnexpectedStatus(status, text)))
    }
}

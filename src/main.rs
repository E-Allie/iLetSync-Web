//! Main application entry point for the iLet Nightscout Synchronizer.
//!
//! This application fetches clinical data from the iLet service and uploads
//! relevant portions (basal and bolus insulin treatments) to a Nightscout site.
//!
//! Configuration options:
//! 1. Command-line arguments: For specifying date ranges.
//! 2. `config.json` file: For iLet and Nightscout credentials, and optionally date ranges.
//!
//! Date range specification is mandatory, either via CLI or config file.
//! CLI arguments for dates take precedence over config file settings.

use std::fs::File;
use std::process::exit;

use anyhow::{Context as AnyhowContext, Result}; // Renamed to avoid conflict with other Context traits
use chrono::{DateTime, Local, TimeZone};
use clap::Parser;

// Local module imports
mod domain;
mod models;

use crate::domain::ilet;
use crate::domain::nightscout;
use crate::models::ilet::iLetSecrets;
use crate::models::nightscout_web::NightscoutSecrets;

/// Defines command-line arguments accepted by the application.
#[derive(Parser, Debug)]
#[clap(
    author = "Your Name / Organization", // Replace with actual author if desired
    version = "0.2.0", // Incremented version for new features
    about = "Fetches iLet data and syncs insulin treatments to a Nightscout site.",
    long_about = "This tool connects to iLet servers to download clinical data for a specified                   date range, then converts and uploads insulin treatment data (basal and bolus)                   to a configured Nightscout instance. Date ranges are mandatory and can be                   provided via command-line arguments or a config.json file. CLI arguments                   override config file settings."
)]
struct CliArgs {
    /// Start date/time for fetching data (RFC3339 format: YYYY-MM-DDTHH:MM:SSZ or YYYY-MM-DDTHH:MM:SS+/-HH:MM)
    #[arg(long, help = "Start date/time for data fetch (RFC3339 format).")]
    start_date: Option<String>,

    /// End date/time for fetching data (RFC3339 format: YYYY-MM-DDTHH:MM:SSZ or YYYY-MM-DDTHH:MM:SS+/-HH:MM)
    #[arg(long, help = "End date/time for data fetch (RFC3339 format).")]
    end_date: Option<String>,

    // TODO: Consider adding CLI flags for other config values (e.g., --config-file path)
}

/// Parses a date string (expected RFC3339 format) into a `DateTime<Local>`.
///
/// # Arguments
/// * `date_str` - The string representation of the date.
/// * `context_msg` - A message describing the source of the date (e.g., "CLI --start-date", "config startDate").
///
/// # Returns
/// A `Result` containing the parsed `DateTime<Local>` or an error message string.
fn parse_date_string(date_str: &str, context_msg: &str) -> Result<DateTime<Local>, String> {
    match DateTime::parse_from_rfc3339(date_str) {
        Ok(dt) => Ok(dt.with_timezone(&Local)),
        Err(e) => Err(format!(
            "Error parsing {} from string '{}': {}. Please use RFC3339 format (e.g., YYYY-MM-DDTHH:MM:SSZ or YYYY-MM-DDTHH:MM:SS+HH:MM).",
            context_msg, date_str, e
        )),
    }
}

/// Main application logic.
fn main() {
    // Parse command-line arguments
    let cli_args = CliArgs::parse();

    // Read configuration from `config.json`
    // The program will exit via `exit(1)` from `read_config` or here if essential config is missing/malformed.
    let (ilet_config, nightscout_config) = match read_config() {
        Ok(configs) => configs,
        Err(e) => {
            eprintln!("Critical: Failed to read or parse configuration: {:?}. Exiting.", e);
            exit(1);
        }
    };

    let effective_start_date: DateTime<Local>;
    let effective_end_date: DateTime<Local>;

    // Determine effective start and end dates based on priority: CLI > Config file
    if let (Some(cli_start_str), Some(cli_end_str)) = (&cli_args.start_date, &cli_args.end_date) {
        println!("Info: Using date range from command-line arguments.");
        match (
            parse_date_string(cli_start_str, "CLI --start-date"),
            parse_date_string(cli_end_str, "CLI --end-date"),
        ) {
            (Ok(start), Ok(end)) => {
                effective_start_date = start;
                effective_end_date = end;
            }
            (Err(parse_err), _) | (_, Err(parse_err)) => {
                eprintln!("{}", parse_err); // Error message already detailed from parse_date_string
                exit(1);
            }
        }
    } else if cli_args.start_date.is_some() || cli_args.end_date.is_some() {
        // Only one of the CLI date arguments was provided
        eprintln!("Error: Both --start-date and --end-date must be provided together via command line if one is specified. Exiting.");
        exit(1);
    } else if let (Some(config_start_str), Some(config_end_str)) =
        (&ilet_config.startDate, &ilet_config.endDate)
    {
        println!("Info: Using date range from config.json.");
        match (
            parse_date_string(config_start_str, "config.json startDate"),
            parse_date_string(config_end_str, "config.json endDate"),
        ) {
            (Ok(start), Ok(end)) => {
                effective_start_date = start;
                effective_end_date = end;
            }
            (Err(parse_err), _) | (_, Err(parse_err)) => {
                eprintln!("{}", parse_err);
                exit(1);
            }
        }
    } else if ilet_config.startDate.is_some() || ilet_config.endDate.is_some() {
        // Only one of the config date arguments was provided
        eprintln!("Error: Both 'startDate' and 'endDate' must be provided together in config.json if one is specified. Exiting.");
        exit(1);
    } else {
        // No dates provided from any source
        eprintln!("Error: Date range not specified. Please provide --start-date and --end-date via command line, or add 'startDate' and 'endDate' to config.json. Exiting.");
        exit(1);
    }

    // Validate that start_date is before end_date
    if effective_start_date >= effective_end_date {
        eprintln!(
            "Error: Start date ({}) must be strictly before end date ({}). Exiting.",
            effective_start_date, effective_end_date
        );
        exit(1);
    }

    println!(
        "Info: Effective date range for iLet data fetch: From {} To {}",
        effective_start_date, effective_end_date
    );

    // Initialize HTTP client
    let http_client = reqwest::blocking::Client::new();

    // Authenticate with iLet and Nightscout, then fetch and process data
    // Errors from these critical steps will cause the program to exit via `exit(1)`.
    let ns_api_token = match nightscout::generate_token(&nightscout_config, &http_client) {
        Ok(token) => token,
        Err(e) => {
            eprintln!("Critical: Failed to generate Nightscout API token: {:?}. Exiting.", e);
            exit(1);
        }
    };
    println!("Info: Successfully obtained Nightscout API token.");

    let ilet_auth_response = match ilet::authenticate_iLet(&ilet_config, &http_client) {
        Ok(response) => response,
        Err(e) => {
            eprintln!("Critical: Failed to authenticate with iLet server: {:?}. Exiting.", e);
            exit(1);
        }
    };
    println!("Info: Successfully authenticated with iLet server.");
    
    let ilet_data_vec = match ilet::grab_iLet_data(
        &http_client,
        ilet_auth_response, // `authenticate_iLet` returns the full response
        ilet_config.serial_number, // `read_config` populates this
        effective_start_date,
        effective_end_date,
    ) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Critical: Failed to grab iLet data: {:?}. Exiting.", e);
            exit(1);
        }
    };

    if ilet_data_vec.is_empty() {
        println!("Info: No iLet data found for the specified period. Nothing to upload to Nightscout.");
    } else {
        println!("Info: Successfully fetched {} records from iLet.", ilet_data_vec.len());
        // Start processing and uploading data to Nightscout
        // `iLet_to_ns_server` handles its own logging for individual uploads.
        nightscout::iLet_to_ns_server(
            ilet_data_vec,
            &http_client,
            ns_api_token,
            nightscout_config,
        );
        println!("Info: Data processing and upload to Nightscout complete (or initiated if running in background).");
    }
    println!("Info: Application finished successfully.");
}

/// Reads and parses the `config.json` file.
///
/// # Returns
/// A `Result` containing a tuple of `(iLetSecrets, NightscoutSecrets)` or an `anyhow::Error`.
fn read_config() -> Result<(iLetSecrets, NightscoutSecrets)> {
    let config_path = "config.json";
    let file_content = match File::open(config_path) {
        Ok(file) => file,
        Err(e) => {
            // Error already includes path, context provides action for user
            return Err(anyhow::Error::new(e).context(format!(
                "Failed to open '{}'. Please ensure the file exists in the current directory.",
                config_path
            )));
        }
    };

    let config_json: serde_json::Value = match serde_json::from_reader(file_content) {
        Ok(value) => value,
        Err(e) => {
            return Err(anyhow::Error::new(e).context(format!(
                "Failed to parse '{}' as JSON. Check for syntax errors.",
                config_path
            )));
        }
    };

    // It's important that iLetSecrets and NightscoutSecrets structs are derived with Deserialize
    let ilet_secrets: iLetSecrets =
        match serde_json::from_value(config_json["iLet"].clone()) {
            Ok(secrets) => secrets,
            Err(e) => {
                return Err(anyhow::Error::new(e).context(
                    "Failed to parse 'iLet' section from config.json. Check structure and fields.",
                ));
            }
        };
    
    let nightscout_secrets: NightscoutSecrets =
        match serde_json::from_value(config_json["Nightscout"].clone()) {
            Ok(secrets) => secrets,
            Err(e) => {
                return Err(anyhow::Error::new(e).context(
                    "Failed to parse 'Nightscout' section from config.json. Check structure and fields.",
                ));
            }
        };

    Ok((ilet_secrets, nightscout_secrets))
}

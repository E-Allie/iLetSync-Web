//! Handles interactions with the Nightscout API.
//!
//! This module includes functions for generating Nightscout API tokens,
//! converting iLet data structures to Nightscout-compatible formats,
//! and posting data to a Nightscout instance.

use anyhow::{anyhow, Context, Result};
use rayon::prelude::*;
use reqwest::blocking::Client;
use rust_decimal::Decimal;
use rust_decimal::prelude::Zero;

use crate::models::ilet::iLetData;
use crate::models::nightscout::{DocumentBase, Food, Treatment}; // NSDocs seems unused now
use crate::models::nightscout_web::{NightscoutBearer, NightscoutSecrets};

/// Generates an API token for accessing the Nightscout service.
///
/// # Arguments
/// * `ns_info` - Configuration details for the Nightscout instance.
/// * `client` - A `reqwest::blocking::Client` for making HTTP requests.
///
/// # Returns
/// A `Result` containing the API token string if successful, or an error.
pub fn generate_token(ns_info: &NightscoutSecrets, client: &Client) -> Result<String> {
    let request_url = format!(
        "{}api/v2/authorization/request/{}",
        ns_info.website, ns_info.permission_role
    );

    client
        .get(&request_url)
        .send()
        .context("Nightscout token request failed to send.")?
        .json::<NightscoutBearer>()
        .map_err(|e| anyhow!("Failed to deserialize Nightscout token response: {}. Response body might not be JSON or not match expected structure.", e))
        .map(|bearer| bearer.token)
        .context("Extracting token from Nightscout response failed.")
}

/// Converts a single iLet data record into Nightscout treatment and food objects.
///
/// Currently, this function primarily focuses on converting insulin delivery data
/// into basal and bolus treatments. Food conversion is stubbed (`None`).
///
/// # Arguments
/// * `ilet_doc` - An iLet data record.
///
/// # Returns
/// A tuple containing:
///   - `Option<Treatment>`: For basal insulin.
///   - `Option<Treatment>`: For bolus insulin.
///   - `Option<Food>`: Currently always `None`.
pub fn ilet_to_ns(ilet_doc: &iLetData) -> (Option<Treatment>, Option<Treatment>, Option<Food>) {
    if ilet_doc.total_insulin_delivered.is_zero() && ilet_doc.meal_dose.is_zero() { // Ensure no insulin or meal dose means no treatment
        return (None, None, None);
    }

    // Base document structure common to treatments
    let doc_base = DocumentBase {
        identifier: None, // Nightscout usually generates this
        date: ilet_doc.app_local_time, // Assuming app_local_time is already in correct NaiveDateTime for NS
        utc_offset: Some(ilet_doc.time_zone_offset_minutes), // Send offset if available
        app: "iLetSync-Rust".to_string(), // Updated app name
        device: Some(format!("iLet SN:{}", ilet_doc.time_zone_offset_minutes)), // Placeholder, ideally real SN
        _id: None, // Nightscout generates this
        srv_created: None,
        subject: None,
        srv_modified: None,
        modified_by: None,
        is_valid: None,
        is_read_only: None,
    };

    let mut basal_treatment: Option<Treatment> = None;
    let mut bolus_treatment: Option<Treatment> = None;

    // Create basal treatment if basal insulin was delivered
    if !ilet_doc.basal_insulin_delivered.is_zero() {
        basal_treatment = Some(Treatment {
            base: doc_base.clone(),
            event_type: Some("Temp Basal".to_string()), // iLet adjustments are like temp basals
            glucose: match ilet_doc.bgm_value {
                x if x > 0 => Some(x.to_string()), // Use BGM if valid
                _ => Some(ilet_doc.cgm_value.to_string()), // Fallback to CGM
            },
            glucose_type: match ilet_doc.bgm_value {
                x if x > 0 => Some("Finger".to_string()), // Standard Nightscout type for manual BGM
                _ => Some("Sensor".to_string()),          // Standard for CGM
            },
            units: Some("mg/dL".to_string()), // TODO: Make this configurable or detect from iLet if possible
            carbs: None,
            protein: None,
            fat: None,
            insulin: Some(ilet_doc.basal_insulin_delivered),
            duration: Some(Decimal::new(5, 0)), // iLet operates on 5-minute cycles
            pre_bolus: None,
            split_now: None,
            split_ext: None,
            percent: None,
            absolute: None, // For temp basals, absolute rate could be here if known
            target_top: None,
            target_bottom: None,
            profile: None,
            reason: None,
            notes: Some(format!("ClinicalDecisionSupportFlags: {}", ilet_doc.clinical_decision_support_flags)),
            entered_by: Some("iLetSync-Rust".to_string()),
        });
    }

    // Calculate non-basal (bolus) insulin
    let bolus_insulin = ilet_doc.total_insulin_delivered - ilet_doc.basal_insulin_delivered;

    if !bolus_insulin.is_zero() && bolus_insulin.is_sign_positive() {
        let mut bolus_base = doc_base.clone(); // Clone for bolus specific event type
        // If there's a meal dose, it's a food bolus, otherwise automated correction
        let event_type_str = if !ilet_doc.meal_dose.is_zero() {
            "Meal Bolus".to_string() 
        } else {
            "Correction Bolus".to_string() // More specific than "Automated Bolus" for Nightscout
        };
        // If meal_dose has carbs associated (not just insulin), add carb info
        let carbs_value = if !ilet_doc.meal_dose.is_zero() {
             Some(ilet_doc.meal_dose) // Assuming meal_dose from iLet might represent carbs or a proxy. Needs clarification.
                                      // If meal_dose is purely insulin, then carbs should be None or handled differently.
        } else {
            None
        };


        bolus_treatment = Some(Treatment {
            base: bolus_base,
            event_type: Some(event_type_str),
            glucose: basal_treatment.as_ref().and_then(|b| b.glucose.clone()), // Reuse glucose from basal if available
            glucose_type: basal_treatment.as_ref().and_then(|b| b.glucose_type.clone()),
            units: basal_treatment.as_ref().and_then(|b| b.units.clone()),
            carbs: carbs_value, // This interpretation of meal_dose needs confirmation
            protein: None,
            fat: None,
            insulin: Some(bolus_insulin),
            duration: Some(Decimal::ZERO), // Boluses are typically 0 duration in Nightscout unless extended
            pre_bolus: None,
            split_now: None,
            split_ext: None,
            percent: None,
            absolute: None,
            target_top: None,
            target_bottom: None,
            profile: None,
            reason: None,
            notes: basal_treatment.as_ref().and_then(|b| b.notes.clone()), // Reuse notes
            entered_by: Some("iLetSync-Rust".to_string()),
        });
    }
    
    // TODO: Implement food data conversion if/when iLet provides clear carbohydrate/food intake data.
    // For now, food is always None.
    let food_entry: Option<Food> = None;

    (basal_treatment, bolus_treatment, food_entry)
}

/// Sends a single Nightscout document (Treatment or Food) to the server.
///
/// # Arguments
/// * `document` - The Nightscout document (as JSON) to send.
/// * `collection_type` - The Nightscout collection type (e.g., "treatments", "food").
/// * `client` - A `reqwest::blocking::Client`.
/// * `ns_token` - The Nightscout API token.
/// * `ns_info` - Nightscout configuration.
///
/// # Returns
/// A `Result` indicating success or failure.
fn send_ns_document<T: serde::Serialize>(
    document: &T,
    collection_type: &str,
    client: &Client,
    ns_token: &str,
    ns_info: &NightscoutSecrets,
) -> Result<()> {
    let url = format!("{}api/v3/{}", ns_info.website, collection_type);
    let response = client
        .post(&url)
        .bearer_auth(ns_token)
        .json(document)
        .send()
        .with_context(|| format!("Failed to send {} document to Nightscout", collection_type))?;

    if response.status().is_success() {
        Ok(())
    } else {
        let status = response.status();
        let text = response.text().unwrap_or_else(|_| "Failed to read error response body".to_string());
        Err(anyhow!(
            "Nightscout API error when sending {} document: {} - {}",
            collection_type, status, text
        ))
    }
}

/// Processes a vector of iLet data, converts them to Nightscout format, and uploads them.
///
/// Data is processed in parallel using Rayon's `par_bridge`. Each iLet record can result
/// in a basal treatment and/or a bolus treatment. Each of these is sent as an
/// individual request to the Nightscout server. This is per the existing constraint
/// that bulk uploads of treatments were not reliably working with the API.
///
/// # Arguments
/// * `ilet_data` - A vector of iLet data records.
/// * `client` - A `reqwest::blocking::Client`.
/// * `ns_token` - The Nightscout API token.
/// * `ns_info` - Nightscout configuration.
pub fn iLet_to_ns_server(
    ilet_data: Vec<iLetData>,
    client: &Client,
    ns_token: String, // Takes ownership as it's used across parallel tasks
    ns_info: NightscoutSecrets, // Takes ownership
) {
    // Using par_bridge for iterators when the number of items isn't fixed or easily chunked for rayon's par_chunks.
    // Here, each ilet_doc can produce 0 to 2 treatments, so par_bridge is suitable.
    ilet_data
        .par_iter() // Process each iLetData item in parallel
        .for_each(|ilet_doc| {
            let (basal, bolus, food) = ilet_to_ns(ilet_doc); // Pass by reference

            if let Some(ref basal_treatment) = basal {
                if let Err(e) = send_ns_document(basal_treatment, "treatments", client, &ns_token, &ns_info) {
                    eprintln!("Error sending basal treatment for iLet time {}: {:?}", ilet_doc.app_local_time, e);
                }
            }

            if let Some(ref bolus_treatment) = bolus {
                if let Err(e) = send_ns_document(bolus_treatment, "treatments", client, &ns_token, &ns_info) {
                    eprintln!("Error sending bolus treatment for iLet time {}: {:?}", ilet_doc.app_local_time, e);
                }
            }

            if let Some(ref food_entry) = food {
                 // This block is currently dormant as food_entry is always None.
                 // If food entries become active, uncomment and test.
                if let Err(e) = send_ns_document(food_entry, "food", client, &ns_token, &ns_info) {
                    eprintln!("Error sending food item for iLet time {}: {:?}", ilet_doc.app_local_time, e);
                }
            }
        });
    println!("Finished processing and sending iLet data to Nightscout.");
}

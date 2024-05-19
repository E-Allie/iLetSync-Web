use anyhow::{anyhow, Result};
use rayon::prelude::*;
use reqwest::blocking::Client;
use reqwest::RequestBuilder;
use rust_decimal::prelude::Zero;
use serde_json::json;

use crate::models::ilet::iLetData;
use crate::models::nightscout::{DocumentBase, NSDocs};
use crate::models::nightscout::Treatment;
use crate::models::nightscout_web::{NightscoutBearer, NightscoutSecrets};

pub fn generate_token(ns_info: &NightscoutSecrets, client: &Client) -> Result<String> {
    let bearer_req = client.get(ns_info.website.clone() + "api/v2/authorization/request/" + &ns_info.permission_role.clone());

    //The response is a json containing "token"
    match bearer_req.send() {
        Ok(resp) => {
            Ok(resp.json::<NightscoutBearer>()?.token)
        }
        Err(e) => return Err(anyhow!("Nightscout Initial Request Unexpectedly Failed! {:?}", e)),
    }
}

pub fn iLet_to_ns(iLet_doc: iLetData) -> [Option<NSDocs>; 2] {
    if iLet_doc.total_insulin_delivered.is_zero() {
        return [None, None];
    } else {

        let doc_base = DocumentBase {
            identifier: None,
            date: iLet_doc.app_local_time.clone(),
            utc_offset: None,
            app: "iLetSync-Web".to_string(),
            device: Some("iLet".to_string()),    //TODO: serial number
            _id: None,
            srv_created: None,
            subject: None,
            srv_modified: None,
            modified_by: None,
            is_valid: None,
            is_read_only: None,
        };

        let basal = Treatment {
            base: doc_base.clone(),
            event_type: Some("Basal".to_string()),
            glucose: match iLet_doc.bgm_value {
                -1 => Some(iLet_doc.cgm_value.to_string()),
                _  => Some(iLet_doc.bgm_value.to_string())
            },
            glucose_type: match iLet_doc.bgm_value {
                -1 => Some("Sensor".to_string()),
                _  => Some("Manual".to_string())
            },
            units: Some("mg/dl".to_string()),   //TODO!: User input, or see if can grab from iLet server
            carbs: None,
            protein: None,
            fat: None,
            insulin: Some(iLet_doc.basal_insulin_delivered),
            duration: None,
            pre_bolus: None,
            split_now: None,
            split_ext: None,
            percent: None,
            absolute: None,
            target_top: None,
            target_bottom: None,
            profile: None,
            reason: None,
            notes: None,
            entered_by: None,
        };

        let mut bolus = basal.clone();
        bolus.insulin = Some(iLet_doc.total_insulin_delivered - iLet_doc.basal_insulin_delivered);

        if iLet_doc.meal_dose.is_zero() {
            //Normal Basal/Bolus
            if iLet_doc.total_insulin_delivered == iLet_doc.basal_insulin_delivered {
                return [Some(NSDocs::TreatmentDoc(basal)), None]
            } else {
                bolus.event_type = Some("Automated Bolus".to_string());
                return [Some(NSDocs::TreatmentDoc(basal)), Some(NSDocs::TreatmentDoc(bolus))];
            }
        } else {
            //Account for food
            bolus.event_type = Some("Food Bolus".to_string());
            return [Some(NSDocs::TreatmentDoc(basal)), Some(NSDocs::TreatmentDoc(bolus))];
        }
    }
}

pub fn iLet_to_ns_server(iLet_data: Vec<iLetData>, client: &Client, ns_token: String, ns_info: NightscoutSecrets) {

    let res = iLet_data.into_par_iter()
        .map(|data| (iLet_to_ns(data)
            .par_iter()
            .for_each_with(client, |client, maybe_doc| {
                match maybe_doc {
                    None => {}
                    Some(doc) => {
                        let collection: &str = match doc {
                            NSDocs::EntryDoc(_) => {"entries"}
                            NSDocs::FoodDoc(_) => {"food"}
                            NSDocs::TreatmentDoc(_) => {"treatments"}
                        };
                        //TODO! Collect Errors better
                        let res = client.post(ns_info.website.clone() + "api/v3/" + collection)
                            .bearer_auth(&ns_token)
                            .header("Content-Type", "application/json")
                            .json(doc)
                            .send()
                            .unwrap();
                    }
                }
            })));

    res.collect()
}


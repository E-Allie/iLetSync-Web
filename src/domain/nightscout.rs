use anyhow::{anyhow, Result};
use rayon::prelude::*;
use reqwest::blocking::{Client, Request, RequestBuilder};
use rust_decimal::Decimal;
use rust_decimal::prelude::Zero;

use crate::models::ilet::iLetData;
use crate::models::nightscout::{DocumentBase, NSDocs};
use crate::models::nightscout::{Food, Treatment};
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

//Do to the current nature of iLet document reading,
//elem 0 and 1 will ALWAYS be specifically Option<NSDocs::TreatmentDoc>,
//TODO! Design choice on how to do food, elem 2
pub fn iLet_to_ns(iLet_doc: iLetData) -> (Option<Treatment>, Option<Treatment>, Option<Food>) {
    if iLet_doc.total_insulin_delivered.is_zero() {
        return (None, None, None);
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
            duration: Some(Decimal::new(5,0)), //For all iLet basals, the duration is technically the 5 minute loop
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
                (Some(basal), None, None)
            } else {
                bolus.event_type = Some("Automated Bolus".to_string());
                (Some(basal), Some(bolus), None)
            }
        } else {
            //Account for food
            bolus.event_type = Some("Food Bolus".to_string());
            (Some(basal), Some(bolus), None)
        }
    }
}

pub fn iLet_to_ns_server(iLet_data: Vec<iLetData>, client: &Client, ns_token: String, ns_info: NightscoutSecrets) {

    // Process in chunks to balance parallelism with connection reuse
    iLet_data
        .par_chunks(1000) // Process 1000 items per thread
        .for_each(|chunk| {
            chunk.iter().for_each(|ilet_doc| {
                let (basal, bolus, food) = iLet_to_ns(ilet_doc.clone());
                
                // Send treatments
                if let Some(basal_treatment) = basal {
                    let result = client
                        .post(format!("{}api/v3/treatments", ns_info.website))
                        .bearer_auth(&ns_token)
                        .json(&basal_treatment)
                        .send();

                    if let Err(e) = result {
                        eprintln!("Error sending basal treatment: {:?}", e);
                    }
                }

                if let Some(bolus_treatment) = bolus {
                    let result = client
                        .post(format!("{}api/v3/treatments", ns_info.website))
                        .bearer_auth(&ns_token)
                        .json(&bolus_treatment)
                        .send();

                    if let Err(e) = result {
                        eprintln!("Error sending bolus treatment: {:?}", e);
                    }
                }

                // Send food
                if let Some(food_entry) = food {
                    let result = client
                        .post(format!("{}api/v3/food", ns_info.website))
                        .bearer_auth(&ns_token)
                        .json(&food_entry)
                        .send();

                    if let Err(e) = result {
                        eprintln!("Error sending food item: {:?}", e);
                    }
                }
            });
        });

    /*iLet_data.par_iter()
        .flat_map(|&iLet_doc| {
            iLet_to_ns(iLet_doc)
        })
        .map(|maybe_doc| {
            match maybe_doc {  }
        })
        .collect();

    iLet_data.into_par_iter()
        .for_each(|data| iLet_to_ns(data)
            .par_iter()
            .map_with(client, |client, maybe_doc| {
                let mut req_builder: RequestBuilder = RequestBuilder::default();
                match maybe_doc {
                    None => {}
                    Some(doc) => {
                        let collection: &str = match doc {
                            NSDocs::EntryDoc(_) => { "entries" }
                            NSDocs::FoodDoc(_) => { "food" }
                            NSDocs::TreatmentDoc(_) => { "treatments" }
                        };
/////3 vec collections
                    }
                }
                req_builder
            }).collect_into_vec(&mut req_vec));*/



    //todo!()


    /*let res = iLet_data.into_par_iter()
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
                        client.post(ns_info.website.clone() + "api/v3/" + collection)
                            .bearer_auth(&ns_token)
                            //.header("Content-Type", "application/json")
                            .json(doc)
                            .send()
                            .unwrap();
                    }
                }
            })));

    res.collect()*/
}


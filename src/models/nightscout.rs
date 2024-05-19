use chrono::{DateTime, Duration, Local};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, DurationSeconds, serde_as};
use serde_with::skip_serializing_none;
use uuid::Uuid;

//A module describing the structs of the NS v3 API
//Refer to the NS API descriptions for more information, the names are equivalent.

//Implementation Decision: This will internally treat ALL numbers as arbitrary precision decimals, since the Nightscout API only dictates the "number" type, even if a smaller integer might seemingly encapsulate all possible values.
//There is an exception for specific types of data, such as dates or UUIDs.


#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) enum NSDocs {
    #[serde(untagged)]
    EntryDoc(Entry),
    #[serde(untagged)]
    FoodDoc(Food),
    #[serde(untagged)]
    TreatmentDoc(Treatment),
}

//Shared values for all documents
#[allow(non_camel_case_types)]
#[skip_serializing_none]
#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DocumentBase {
    pub(crate) identifier: Option<String>,           //Technically optional, but best practices dictate maintaining it.
    pub(crate) date: String,      //Consider Replacing
    pub(crate) utc_offset: Option<i16>,     //Consider replacing with offset type/building from date
    pub(crate) app: String,                //Immutable by client, should ALWAYS be "iLetSync-Web"
    pub(crate) device: Option<String>,     //Immutable by client, should ALWAYS be "iLet" or "iLet Bionic Pancreas" etc.
    pub(crate) _id: Option<String>,
    pub(crate) srv_created: Option<DateTime<Local>>,  //Immutable by client, NS creation time
    pub(crate) subject: Option<String>,             //Immutable by client
    pub(crate) srv_modified: Option<DateTime<Local>>, //Immutable by client
    pub(crate) modified_by: Option<String>,           //Immutable by client
    pub(crate) is_valid: Option<bool>,                //Immutable by client
    pub(crate) is_read_only: Option<bool>,
}

//Blood glucose measurements and CGM calibrations
#[allow(non_camel_case_types)]
#[skip_serializing_none]
#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Entry {
    #[serde(flatten)]
    base: DocumentBase,
    #[serde(rename = "type")]
    _type: Option<String>,       //"type" is a keyword
    sgv: Option<Decimal>,
    direction: Option<String>,
    noise: Option<Decimal>,
    filtered: Option<Decimal>,
    unfiltered: Option<Decimal>,
    rssi: Option<Decimal>,
    units: String,              //Technically optional, will be made mandatory due to best practices.
}


//Nutritional values of food
//TODO: quickpick?
#[allow(non_camel_case_types)]
#[skip_serializing_none]
#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Food {
    #[serde(flatten)]
    base: DocumentBase,
    food: Option<String>,
    category: Option<String>,
    subcategory: Option<String>,
    name: Option<String>,
    #[serde(with = "rust_decimal::serde::arbitrary_precision_option")]
    portion: Option<Decimal>,
    unit: Option<String>,
    #[serde(with = "rust_decimal::serde::arbitrary_precision_option")]
    carbs: Option<Decimal>,
    #[serde(with = "rust_decimal::serde::arbitrary_precision_option")]
    fat: Option<Decimal>,
    #[serde(with = "rust_decimal::serde::arbitrary_precision_option")]
    protein: Option<Decimal>,
    #[serde(with = "rust_decimal::serde::arbitrary_precision_option")]
    energy: Option<Decimal>,
    #[serde(with = "rust_decimal::serde::arbitrary_precision_option")]
    gi: Option<Decimal>,
    hide_after_use: Option<bool>,
    hidden: Option<bool>,
    #[serde(with = "rust_decimal::serde::arbitrary_precision_option")]
    position: Option<Decimal>,
    #[serde(with = "rust_decimal::serde::arbitrary_precision_option")]
    portions: Option<Decimal>,
    //foods
}

//T1D Compensation Action
#[allow(non_camel_case_types)]
#[skip_serializing_none]
#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Treatment {
    #[serde(flatten)]
    pub(crate) base: DocumentBase,
    pub(crate) event_type: Option<String>,           //Immutable by client
    pub(crate) glucose: Option<String>,
    pub(crate) glucose_type: Option<String>,
    pub(crate) units: Option<String>,
    #[serde(with = "rust_decimal::serde::arbitrary_precision_option")]        //Serde_as can't be given a module path
    pub(crate) carbs: Option<Decimal>,
    #[serde(with = "rust_decimal::serde::arbitrary_precision_option")]
    pub(crate) protein: Option<Decimal>,
    #[serde(with = "rust_decimal::serde::arbitrary_precision_option")]
    pub(crate) fat: Option<Decimal>,
    #[serde(with = "rust_decimal::serde::arbitrary_precision_option")]
    pub(crate) insulin: Option<Decimal>,
    #[serde_as(as = "Option<DurationSeconds<i64>>")]
    pub(crate) duration: Option<Duration>,         //TODO: Come back to this, nightscout expects minutes
    #[serde_as(as = "Option<DurationSeconds<i64>>")]
    pub(crate) pre_bolus: Option<Duration>,
    #[serde(with = "rust_decimal::serde::arbitrary_precision_option")]
    pub(crate) split_now: Option<Decimal>,
    #[serde(with = "rust_decimal::serde::arbitrary_precision_option")]
    pub(crate) split_ext: Option<Decimal>,
    #[serde(with = "rust_decimal::serde::arbitrary_precision_option")]
    pub(crate) percent: Option<Decimal>,
    #[serde(with = "rust_decimal::serde::arbitrary_precision_option")]
    pub(crate) absolute: Option<Decimal>,
    #[serde(with = "rust_decimal::serde::arbitrary_precision_option")]
    pub(crate) target_top: Option<Decimal>,
    #[serde(with = "rust_decimal::serde::arbitrary_precision_option")]
    pub(crate) target_bottom: Option<Decimal>,
    pub(crate) profile: Option<String>,
    pub(crate) reason: Option<String>,
    pub(crate) notes: Option<String>,
    pub(crate) entered_by: Option<String>,
}

use serde::{Deserialize, Serialize};
use crate::models::nightscout;

#[derive(Serialize, Deserialize)]
pub(crate) struct NightscoutSecrets {
    pub(crate) website: String,
    pub(crate) permission_role: String,
}

#[derive(Deserialize)]
pub(crate) struct NightscoutBearer {
    pub(crate) token: String,
}

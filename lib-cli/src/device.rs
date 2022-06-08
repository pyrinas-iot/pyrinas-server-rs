use serde::{Deserialize, Serialize};
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceCert {
    pub client_id: String,
    pub ca_cert: String,
    pub private_key: String,
    pub public_key: String,
}

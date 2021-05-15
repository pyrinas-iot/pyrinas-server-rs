use serde::Deserialize;
#[derive(Debug, Deserialize)]
pub struct Config {
    pub domain: String,
    pub organization: String,
    pub country: String,
}

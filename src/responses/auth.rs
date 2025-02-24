use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(uniffi::Enum)]
#[derive(Clone, Serialize_repr, Deserialize_repr, Debug)]
#[repr(u8)]
pub enum AuthVersion {
    V1 = 1,
    V2 = 2,
}

#[derive(uniffi::Record)]
#[derive(Deserialize, Debug)]
pub struct AuthInfoResponse {
    pub email: String,
    #[serde(rename = "authVersion")]
    pub auth_version: AuthVersion,
    pub salt: String,
    pub id: i64,
}
use std::fmt;

use crate::response_struct;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

use super::bool_from_int;

#[derive(uniffi::Enum)]
#[derive(Clone, Serialize_repr, Deserialize_repr, Debug, Copy)]
#[repr(u8)]
pub enum AuthVersion {
    V1 = 1,
    V2 = 2,
}

impl fmt::Display for AuthVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

response_struct! {
    AuthInfoResponse {
        email: String,
        auth_version: AuthVersion,
        salt: String,
        id: i64,
    }

    LoginResponse {
        api_key: String,
        master_keys: String,
        public_key: String,
        private_key: String,
    }
    
    UserInfoResponse {
        id: i64,
        email: String,
        #[serde(deserialize_with = "bool_from_int")]
        is_premium: bool,
        max_storage: i64,
        storage_used: i64,
        #[serde(rename = "avatarURL")]
        avatar_url: String,
        #[serde(rename = "baseFolderUUID")]
        base_folder_uuid: String,
    }
}


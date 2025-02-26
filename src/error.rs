use crate::responses::auth::AuthVersion;

#[derive(Debug, thiserror::Error)]
#[derive(uniffi::Error)]
pub enum FilenSDKError {
    #[error("Unsupported Auth Version: {version}")]
    AuthVersionError { version: AuthVersion},

    #[error("Error handling the request: {err_str}")]
    ReqwestError { err_str: String },

    #[error("Error parsing JSON: {err_str} \nWith message: {err_msg}")]
    SerdeJsonError { err_msg: String, err_str: String },

    #[error("API Error code: {message}")]
    APIError {
        message: String,
        code: Option<String>,
    },

    #[error("Not logged in")]
    NoCredentials,

    #[error("Unknown Error: {err_str}")]
    UnknownError { err_str: String },
}
use std::convert::Infallible;

use crate::{crypto::CryptoError, responses::auth::AuthVersion};

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

    #[error("File does not exist: {file}")]
    FileDoesNotExist { file: String },

    #[error("Error encrypting file: {err_str}")]
    EncryptionError { err_str: String },

    #[error("Error uploading file: {err_str}")]
    UploadError { err_str: String },

    #[error("Error downloading file: {err_str}")]
    DownloadError { err_str: String },

    #[error("Error creating string from UTF8 data: {err_str}")]
    FromUtf8Error { err_str: String },

    #[error("Error creating path: {path}")]
    InvalidPath { path: String },

    #[error("Error creating path: {path}")]
    PathIsDirectory { path: String },

    #[error("Stream Ended")]
    StreamEnded,

    #[error("Unknown Error: {err_str}")]
    UnknownError { err_str: String },
}

impl From<CryptoError> for FilenSDKError {
    fn from(err: CryptoError) -> Self {
        FilenSDKError::EncryptionError { err_str: err.to_string() }
    }
}

impl From<std::string::FromUtf8Error> for FilenSDKError {
    fn from(err: std::string::FromUtf8Error) -> Self {
        FilenSDKError::FromUtf8Error { err_str: err.to_string() }
    }
}

impl From<serde_json::Error> for FilenSDKError {
    fn from(err: serde_json::Error) -> Self {
        FilenSDKError::SerdeJsonError { err_str: "".to_string(), err_msg: err.to_string() }
    }
}

impl From<Infallible> for FilenSDKError {
    fn from(err: Infallible) -> Self {
        FilenSDKError::UnknownError { err_str: err.to_string() }
    }
}

impl From<std::io::Error> for FilenSDKError {
    fn from(err: std::io::Error) -> Self {
        FilenSDKError::UnknownError { err_str: err.to_string() }
    }
}

impl From<reqwest::Error> for FilenSDKError {
    fn from(err: reqwest::Error) -> Self {
        FilenSDKError::ReqwestError { err_str: err.to_string() }
    }
}


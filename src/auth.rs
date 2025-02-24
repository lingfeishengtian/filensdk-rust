use crate::{httpclient::{make_post_request, FilenURL}, responses::auth::AuthInfoResponse, FilenSDK};

// FilenError is a custom error type that is defined in the root of the crate
#[derive(Debug, thiserror::Error)]
#[derive(uniffi::Error)]
pub enum FilenSDKError {
    #[error("Unknown Error: {err_str}")]
    UnknownError { err_str: String },
}

#[uniffi::export]
impl FilenSDK {
    pub async fn retrieve_auth_info(&self, email: &str) -> Result<Option<AuthInfoResponse>, FilenSDKError> {
        match make_post_request(FilenURL::baseUrl("/v3/auth/info".to_string()), None, None, Some([("email", email)].iter().cloned().collect())) {
            Ok(response) => Ok(response.data),
            Err(e) => Err(FilenSDKError::UnknownError { err_str: e.to_string() })
        }
    }
}
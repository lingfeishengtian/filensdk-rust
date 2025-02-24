use crate::{credentials::SDKCreds, error::FilenSDKError, httpclient::{http_none, make_get_request, make_post_request, FilenResponse, FilenURL}, requests::auth::{AuthInfoRequest, LoginRequest}, responses::auth::{self, AuthInfoResponse, AuthVersion, LoginResponse, UserInfoResponse}, FilenSDK};

#[uniffi::export]
impl FilenSDK {
    pub async fn retrieve_auth_info(&self, email: &str) -> Result<AuthInfoResponse, FilenSDKError> {
        make_post_request(
            FilenURL::baseUrl("/v3/auth/info".to_string()), 
            None, 
            None, 
            Some(
                AuthInfoRequest {
                    email: email.to_string()
                }
            )
        )
    }
}
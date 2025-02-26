use crate::{error::FilenSDKError, httpclient::{make_request, Endpoints}, requests::auth::AuthInfoRequest, responses::auth::AuthInfoResponse, FilenSDK};

#[uniffi::export]
impl FilenSDK {
    pub async fn retrieve_auth_info(&self, email: &str) -> Result<AuthInfoResponse, FilenSDKError> {
        make_request(
            Endpoints::AuthInfo,
            Some(&self.client.clone()),
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
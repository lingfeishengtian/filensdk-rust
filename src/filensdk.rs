use std::sync::{Arc, Mutex};

use tokio::sync::Semaphore;

use crate::{
    credentials::SDKCreds,
    error::FilenSDKError,
    httpclient::{http_none, make_request, Endpoints},
    requests::auth::LoginRequest,
    responses::auth::{AuthVersion, LoginResponse, UserInfoResponse}
};

#[derive(uniffi::Object)]
pub struct FilenSDK {
    credentials: Arc<Mutex<Option<SDKCreds>>>,
    /*
    Temporarily keep the decrypt semaphore here although it most likely will not be needed
    since the crypto module uses ring, being so fast that scheduling tasks actually slows
    the process down. For example, in decryption, in-memory decryption removes the need to
    read from a file after decrypting, saving memory and speed. 
     */ 
    pub decrypt_semaphore: Arc<Semaphore>,
    pub download_semaphore: Arc<Semaphore>,
    pub client: Arc<reqwest::Client>
}

pub const MAX_DECRYPT_THREADS: usize = 10;
pub const MAX_DOWNLOAD_THREADS: usize = 50;

#[uniffi::export]
impl FilenSDK {
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self { 
            credentials: Arc::new(Mutex::new(None)),
            decrypt_semaphore: Arc::new(Semaphore::new(MAX_DECRYPT_THREADS)),
            download_semaphore: Arc::new(Semaphore::new(MAX_DOWNLOAD_THREADS)),
            client: Arc::new(reqwest::Client::new())
        }
    }

    // Utilize serde to convert the input to JSON String that can be stored locally
    pub fn export_credentials(&self) -> String {
        let creds = self.credentials.lock().unwrap();

        match &*creds {
            Some(creds) => ron::ser::to_string(creds).unwrap(),
            None => String::new()
        }
    }

    pub fn import_credentials(&self, creds: String) {
        let creds: SDKCreds = ron::de::from_str(&creds).unwrap();
        self.credentials.lock().unwrap().replace(creds);
    }

    pub async fn login(&self, email: &str, password: &str, two_factor: Option<String>) -> Result<bool, FilenSDKError>
     {
        // TBH this really isn't async, but support the foreign function interface "calling convention"
        let auth_info = self.retrieve_auth_info(email).await?;
        match auth_info.auth_version {
            AuthVersion::V1 => return Err(FilenSDKError::AuthVersionError { version: auth_info.auth_version }),
            AuthVersion::V2 => (),
        }

        let derived_creds = crate::crypto::password::derive_credentials_from_password(auth_info.auth_version, password, Some(&auth_info.salt));
        let login_response: LoginResponse = make_request(
            Endpoints::Login,
            Some(&self.client.clone()),
            None,
            None, 
            Some(LoginRequest {
                email: email.to_string(),
                password: derived_creds.password,
                two_factor_code: if let Some(code) = two_factor { code } else { "".to_string() },
                auth_version: auth_info.auth_version,
            })
        )?;

        let user_info = user_info_request(&login_response.api_key)?;
        let creds = SDKCreds::new (
            vec![derived_creds.master_key],
            login_response.api_key,
            Some(login_response.public_key),
            Some(login_response.private_key),
            auth_info.auth_version,
            Some(auth_info.id),
            Some(user_info.base_folder_uuid),
        );

        self.credentials.lock().unwrap().replace(creds);

        Ok(true)
    }

    pub fn user_id(&self) -> String {
        let creds = self.credentials.lock().unwrap();
        match &*creds {
            Some(creds) => creds.user_id.unwrap_or(-1).to_string(),
            None => String::new()
        }
    }

    pub fn api_key(&self) -> Result<String, FilenSDKError> {
        let creds = self.credentials.lock().unwrap();
        match &*creds {
            Some(creds) => Ok(creds.api_key.clone()),
            None => Err(FilenSDKError::NoCredentials)
        }
    }
}

fn user_info_request(api_key: &str) -> Result<UserInfoResponse, FilenSDKError> {
    make_request(
        Endpoints::UserInfo,
        None,
        None,
        Some(api_key),
        http_none()
    )
}
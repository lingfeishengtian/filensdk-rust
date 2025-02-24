use serde::{Deserialize, Serialize};
use crate::responses::auth::AuthVersion;

#[derive(uniffi::Record)]
#[derive(Clone, Serialize, Deserialize)]
pub struct SDKCreds {
    pub master_keys: Vec<String>,
    pub api_key: String,
    pub public_key: Option<String>,
    pub private_key: Option<String>,
    pub auth_version: AuthVersion,
    pub user_id: Option<i64>,
    pub base_folder_uuid: Option<String>,
}

impl SDKCreds {
    pub fn new(
        master_keys: Vec<String>,
        api_key: String,
        public_key: Option<String>,
        private_key: Option<String>,
        auth_version: AuthVersion,
        user_id: Option<i64>,
        base_folder_uuid: Option<String>,
    ) -> Self {
        SDKCreds {
            master_keys,
            api_key,
            public_key,
            private_key,
            auth_version,
            user_id,
            base_folder_uuid,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::credentials::{SDKCreds, AuthVersion};
    use crate::filensdk::FilenSDK;

    #[test]
    fn test_cred_set() {
        // let sdk = FilenSDK::new();
        // sdk.set_credentials(SDKCreds {
        //     masterKeys: vec!["key1".to_string(), "key2".to_string()],
        //     apiKey: "api_key".to_string(),
        //     publicKey: Some("public_key".to_string()),
        //     privateKey: Some("private_key".to_string()),
        //     authVersion: AuthVersion::V1,
        //     userId: Some(1),
        //     baseFolderUUID: Some("uuid".to_string()),
        // });

        // let creds = sdk.get_credentials().unwrap();
        // assert_eq!(creds.masterKeys, vec!["key1".to_string(), "key2".to_string()]);
    }
}
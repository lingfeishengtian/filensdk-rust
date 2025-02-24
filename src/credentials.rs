use serde::{Deserialize, Serialize};
use crate::responses::auth::AuthVersion;

#[derive(uniffi::Record)]
#[derive(Clone, Serialize, Deserialize)]
pub struct SDKCreds {
    masterKeys: Vec<String>,
    apiKey: String,
    publicKey: Option<String>,
    privateKey: Option<String>,
    authVersion: AuthVersion,
    userId: Option<i32>,
    baseFolderUUID: Option<String>,
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
use std::sync::{Arc, Mutex};

use serde::Serialize;

use crate::credentials::SDKCreds;

#[derive(uniffi::Object)]
pub struct FilenSDK {
    credentials: Arc<Mutex<Option<SDKCreds>>>
}

#[uniffi::export]
impl FilenSDK {
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self { credentials: Arc::new(Mutex::new(None)) }
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
}
use uniffi_shared_tokio_runtime_proc::uniffi_async_export;

use crate::{httpclient::make_request, requests::fs::{FileInfoBody, FileMetadata}, responses::auth::AuthVersion, FilenSDK};

#[derive(uniffi::Record, Debug, Clone)]
pub struct FilenFileDetailed {
    pub uuid: String,
    pub region: String,
    pub bucket: String,
    pub name: String,
    pub size: u64,
    pub mime: String,
    pub key: Vec<u8>,
    pub last_modified: Option<i64>,
    pub parent: String,
    pub versioned: Option<bool>,
    pub trash: bool,
    pub version: AuthVersion
}

#[uniffi_async_export]
impl FilenSDK {
    /// Retrieves detailed information about a file. Under the hood, it retrieves the encrypted
    /// information about the file, decrypts it with your master key, and returns the decrypted
    /// information.
    pub async fn file_info(
        &self,
        uuid: String,
    ) -> Result<FilenFileDetailed, crate::error::FilenSDKError> {
        make_request(
            crate::httpclient::Endpoints::FileInfo, 
            Some(&self.client.clone()), 
            None, 
            Some(&self.api_key()?), 
            Some(FileInfoBody { uuid })
        ).await.and_then(|x| self.decrypt_get_response(x))
    }

    pub async fn encrypted_file_info(
        &self,
        uuid: String,
    ) -> Result<crate::responses::fs::FileGetResponse, crate::error::FilenSDKError> {
        make_request(
            crate::httpclient::Endpoints::FileInfo, 
            Some(&self.client.clone()), 
            None, 
            Some(&self.api_key()?), 
            Some(FileInfoBody { uuid })
        ).await
    }
}

impl FilenSDK {
    pub fn decrypt_metadata(metadata: String, key: String) -> Result<FileMetadata, crate::error::FilenSDKError> {
        let metadata = crate::crypto::metadata::decrypt_metadata(
            &metadata.as_bytes(),
            &key,
        ).unwrap();

        Ok(serde_json::from_str(&String::from_utf8(metadata)?)?)
    }

    pub fn decrypt_get_response(
        &self,
        response: crate::responses::fs::FileGetResponse,
    ) -> Result<FilenFileDetailed, crate::error::FilenSDKError> {
        let metadata = Self::decrypt_metadata(response.metadata, self.master_key()?)?;

        Ok(FilenFileDetailed {
            uuid: response.uuid,
            region: response.region,
            bucket: response.bucket,
            name: metadata.name,
            size: metadata.size.unwrap(),
            mime: metadata.mime.unwrap(),
            key: metadata.key,
            last_modified: metadata.last_modified,
            parent: response.parent,
            versioned: Some(response.versioned),
            trash: response.trash,
            version: response.version,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_info() {
        dotenv::dotenv().ok();
        let filensdk = FilenSDK::new();
        filensdk.import_credentials(dotenv::var("TEST_CRED_IMPORT").unwrap());

        let uuid = dotenv::var("TEST_UUID").unwrap();

        let response = filensdk.file_info_blocking(uuid);
        assert!(response.is_ok());

        let dotenv_bucket = dotenv::var("TEST_BUCKET").unwrap();
        let dotenv_region = dotenv::var("TEST_REGION").unwrap();

        let response = response.unwrap();
        assert_eq!(response.bucket, dotenv_bucket);
        assert_eq!(response.region, dotenv_region);

        // Test validity of decrypted response
        assert_eq!(response.bucket, dotenv_bucket);
        assert_eq!(response.region, dotenv_region);

        println!("{:?}", response);
    }
}
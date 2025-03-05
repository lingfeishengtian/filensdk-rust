use crate::{httpclient::make_request, requests::fs::{FileInfoBody, FileMetadata}, FilenSDK};

#[derive(uniffi::Record, Debug)]
pub struct DecryptedFileGetResponse {
    pub uuid: String,
    pub region: String,
    pub bucket: String,
    pub name: String,
    pub size: i64,
    pub mime: String,
    pub key: Vec<u8>,
    pub last_modified: Option<i64>,
    pub parent: String,
    pub versioned: bool,
    pub trash: bool,
    pub version: i64,
}

impl FilenSDK {
    pub async fn file_info(
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

    pub fn decrypt_metadata(&self, metadata: String, key: String) -> Result<FileMetadata, crate::error::FilenSDKError> {
        let metadata = crate::crypto::metadata::decrypt_metadata(
            &metadata.as_bytes(),
            &key,
        ).unwrap();

        Ok(serde_json::from_str(&String::from_utf8(metadata)?)?)
    }

    pub fn decrypt_get_response(
        &self,
        response: crate::responses::fs::FileGetResponse,
    ) -> Result<DecryptedFileGetResponse, crate::error::FilenSDKError> {
        let metadata = self.decrypt_metadata(response.metadata, self.master_key()?)?;

        Ok(DecryptedFileGetResponse {
            uuid: response.uuid,
            region: response.region,
            bucket: response.bucket,
            name: metadata.name,
            size: metadata.size.unwrap() as i64,
            mime: metadata.mime.unwrap(),
            key: metadata.key,
            last_modified: metadata.last_modified,
            parent: response.parent,
            versioned: response.versioned,
            trash: response.trash,
            version: response.version,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::crypto::metadata::decrypt_metadata;

    use super::*;

    #[async_std::test]
    async fn test_file_info() {
        dotenv::dotenv().ok();
        let filensdk = FilenSDK::new();
        filensdk.import_credentials(dotenv::var("TEST_CRED_IMPORT").unwrap());

        let uuid = dotenv::var("TEST_UUID").unwrap();

        let response = filensdk.file_info(uuid);
        assert!(response.await.is_ok());

        let dotenv_bucket = dotenv::var("TEST_BUCKET").unwrap();
        let dotenv_region = dotenv::var("TEST_REGION").unwrap();

        let response = response.unwrap();
        assert_eq!(response.bucket, dotenv_bucket);
        assert_eq!(response.region, dotenv_region);

        let encrypted_name = response.name_encrypted.clone();
        let metadata = filensdk.decrypt_metadata(response.metadata.clone(), filensdk.master_key().unwrap()).unwrap();

        // Test validity of decrypted response
        let decrypted_response = filensdk.decrypt_get_response(response).unwrap();
        assert_eq!(decrypted_response.bucket, dotenv_bucket);
        assert_eq!(decrypted_response.region, dotenv_region);

        let string_decrypted_name_from_metadata_key = String::from_utf8(decrypt_metadata(&encrypted_name.as_bytes(), &String::from_utf8(metadata.key).unwrap()).unwrap()).unwrap();
        assert_eq!(decrypted_response.name, string_decrypted_name_from_metadata_key);

        println!("{:?}", decrypted_response);
    }
}
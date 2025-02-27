use crate::{httpclient::make_request, requests::fs::FileInfoBody, FilenSDK};


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
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[async_std::test]
    async fn test_file_info() {
        dotenv::dotenv().ok();
        let filensdk = FilenSDK::new();
        filensdk.import_credentials(dotenv::var("TEST_CRED_IMPORT").unwrap());

        let uuid = dotenv::var("TEST_UUID").unwrap();

        let response = filensdk.file_info(uuid).await;
        assert!(response.is_ok());

        let dotenv_bucket = dotenv::var("TEST_BUCKET").unwrap();
        let dotenv_region = dotenv::var("TEST_REGION").unwrap();

        let response = response.unwrap();
        assert_eq!(response.bucket, dotenv_bucket);
        assert_eq!(response.region, dotenv_region);
    }
}
use std::{future::Future, sync::Arc};

use bytes::{Bytes, BytesMut};

use crate::{error::FilenSDKError, httpclient::{download_into_memory, httpclient::upload_from_memory, FsURL}};

use super::FilenNetInteractionFunctions;


#[derive(Clone)]
pub struct LowDiskInteractionFunctions {
    pub client: Arc<reqwest::Client>,
    pub api_key: String,
    pub should_use_counter_nonce: bool
}


impl FilenNetInteractionFunctions<Bytes> for LowDiskInteractionFunctions {
    fn http_retrieve_data(&self, link: FsURL, _i: u64) -> impl Future<Output = Result<Bytes, FilenSDKError>> + Send {
        async move { download_into_memory(&link, &self.client).await }
    }

    fn decrypt_retrieve_data(&self, data: Bytes) -> BytesMut {
        data.into()
    }

    fn encrypt_data(&self, input_file: &str, i: u64, key: &[u8; 32]) -> (Bytes, String) {
        let (encrypted_data, hash) = crate::crypto::file_encrypt::encrypt_v2_from_file(
            &input_file,
            None,
            key,
            i as usize,
            self.should_use_counter_nonce
        )
        .unwrap();
        (encrypted_data.unwrap(), hash)
    }

    fn http_upload_data(&self, link: FsURL, data: Bytes) -> impl Future<Output = Result<(), FilenSDKError>> + Send {
        async move { upload_from_memory(link, &self.client, data, &self.api_key).await.map(|_| ()) }
    }
}

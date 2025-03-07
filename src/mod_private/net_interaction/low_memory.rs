use std::{future::Future, sync::Arc};

use bytes::{Bytes, BytesMut};

use crate::{error::FilenSDKError, httpclient::{download_to_file_streamed, httpclient::upload_from_file, FsURL}};

use super::FilenNetInteractionFunctions;

#[derive(Clone)]
pub struct LowMemoryInteractionFunctions {
    pub client: Arc<reqwest::Client>,
    pub api_key: String,
    pub tmp_dir: String,
    pub should_use_counter_nonce: bool
}


impl FilenNetInteractionFunctions<String> for LowMemoryInteractionFunctions {
    fn http_retrieve_data(&self, link: FsURL, i: u64) -> impl Future<Output = Result<String, FilenSDKError>> + Send {
        let tmp_dir = self.tmp_dir.clone() + "/" + &i.to_string();
        async move { download_to_file_streamed(&link, &self.client, &tmp_dir).await }
    }

    fn decrypt_retrieve_data(&self, data: String) -> BytesMut {
        let file = std::fs::File::open(&data).unwrap();
        let mut reader = std::io::BufReader::new(file);
        let mut buffer = Vec::new();
        std::io::Read::read_to_end(&mut reader, &mut buffer).unwrap();

        // Delete the file after reading
        std::fs::remove_file(&data).unwrap();

        let bytes = Bytes::from(buffer);
        
        bytes.try_into_mut().unwrap()
    }
    
    fn encrypt_data(&self, input_file: &str, i: u64, key: &[u8; 32]) -> (String, String) {
        let output_file = self.tmp_dir.to_string() + "/" + &i.to_string();
        let (_encrypted_data, hash) = crate::crypto::file_encrypt::encrypt_v2_from_file(
            &input_file,
            Some(&output_file),
            key,
            i as usize,
            self.should_use_counter_nonce
        )
        .unwrap();
        (output_file, hash)
    }
    
    fn http_upload_data(&self, link: FsURL, data: String) -> impl Future<Output = Result<(), FilenSDKError>> + Send {
        async move { 
            let fut = upload_from_file(link, &self.client, &data, &self.api_key).await.map(|_| ());
            std::fs::remove_file(&data).unwrap();
            fut
        }
    }
}
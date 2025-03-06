use std::{future::Future, sync::Arc};

use bytes::{Bytes, BytesMut};

use crate::{error::FilenSDKError, httpclient::{download_into_memory, download_to_file_streamed, httpclient::{upload_from_file, upload_from_memory}, FsURL}};


pub trait FilenNetInteractionFunctions<T>: Send + Sync + Clone + 'static {
    fn http_retrieve_data(&self, link: FsURL, i: u64) -> impl Future<Output = Result<T, FilenSDKError>> + Send;
    /// The data retrieved by this method **SHOULD NOT** be decrypted. Rather, this method retrieves the
    /// data into memory for the decryption process. Different methods (streaming vs file) will use the 
    /// data in different ways.
    fn decrypt_retrieve_data(&self, data: T) -> BytesMut;
    /// Encrypt and return the data, along with the encryption hash
    fn encrypt_data(&self, input_file: &str, i: u64, key: &[u8; 32]) -> (T, String);
    fn http_upload_data(&self, link: FsURL, data: T) -> impl Future<Output = Result<(), FilenSDKError>> + Send;
}

#[derive(Clone)]
pub struct LowDiskInteractionFunctions {
    pub client: Arc<reqwest::Client>,
    pub api_key: String,
    pub should_use_counter_nonce: bool
}

#[derive(Clone)]
pub struct LowMemoryInteractionFunctions {
    pub client: Arc<reqwest::Client>,
    pub api_key: String,
    pub tmp_dir: String,
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
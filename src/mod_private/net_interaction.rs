use std::future::Future;

use bytes::BytesMut;

use crate::{error::FilenSDKError, httpclient::FsURL};

mod low_disk;
mod low_memory;

pub use low_disk::LowDiskInteractionFunctions;
pub use low_memory::LowMemoryInteractionFunctions;

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
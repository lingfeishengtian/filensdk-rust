use std::{pin::Pin, sync::{Arc}};

use futures::{lock::Mutex, TryStreamExt};
use futures_core::Stream;
use bytes::Bytes;

use crate::{error::FilenSDKError, FilenSDK};

#[derive(uniffi::Object)]
pub struct FilenDownloadStream {
    internal_stream: Arc<futures::lock::Mutex<Pin<Box<dyn Stream<Item = Result<Bytes, FilenSDKError>> + Send + Sync>>>>,
    filen_sdk: Arc<FilenSDK>,
}

#[uniffi::export]
impl FilenDownloadStream {
    #[uniffi::constructor]
    pub fn new(
        size: u64,
        start_byte: u64,
        filen_sdk: Arc<FilenSDK>,
        region: String,
        bucket: String,
        uuid: String,
        key: String,
    ) -> Self {
        Self {
            internal_stream: Arc::new(Mutex::new(Box::pin(filen_sdk.read_ahead_download_stream(
                size,
                start_byte,
                region,
                bucket,
                uuid,
                key,
            )))),
            filen_sdk,
        }
    }

    #[uniffi::constructor]
    pub async fn new_from_uuid(
        uuid: &str,
        filen_sdk: Arc<FilenSDK>,
        start_byte: u64,
    ) -> Result<Self, FilenSDKError> {
        let info = filen_sdk.file_info(uuid.to_owned()).await?;

        Ok(Self::new(
            info.size,
            start_byte,
            filen_sdk,
            info.region,
            info.bucket,
            uuid.to_string(),
            String::from_utf8(info.key).unwrap(),
        ))
    }

    #[uniffi::method(name = "next")]
    pub fn next_blocking(&self) -> Result<Vec<u8>, FilenSDKError> {
        self.filen_sdk.tokio_runtime.lock().unwrap().as_ref().unwrap().block_on(self.next())
    }
}

impl FilenDownloadStream {
    pub async fn next(&self) -> Result<Vec<u8>, FilenSDKError> {
        let mut stream = self.internal_stream.lock().await;
        let mut stream = stream.as_mut();
        let next = stream.try_next().await;
        if let Some(next) = next?.map(|b| b.to_vec()) {
            Ok(next)
        } else {
            Err(FilenSDKError::StreamEnded)
        }
    }
}
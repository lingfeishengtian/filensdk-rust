use std::{future::Future, sync::Arc, time::Duration};

use bytes::{Bytes, BytesMut};

use super::DOWNLOAD_RETRIES;
use crate::{error::FilenSDKError, httpclient::{download_into_memory, download_to_file_streamed, FsURL}, FilenSDK};

pub trait DownloadFunctions<T>: Send + Sync + Clone + 'static {
    fn http_retrieve_data(&self, link: FsURL, i: u64) -> impl Future<Output = Result<T, FilenSDKError>> + Send;
    fn decrypt_retrieve_data(&self, data: T) -> BytesMut;
}

#[derive(Clone)]
pub struct LowDiskDownloadFunctions {
    pub client: Arc<reqwest::Client>,
}
#[derive(Clone)]
pub struct LowMemoryDownloadFunctions {
    pub client: Arc<reqwest::Client>,
    pub tmp_dir: String,
}

impl DownloadFunctions<Bytes> for LowDiskDownloadFunctions {
    fn http_retrieve_data(&self, link: FsURL, i: u64) -> impl Future<Output = Result<Bytes, FilenSDKError>> + Send {
        async move { download_into_memory(&link, &self.client).await }
    }

    fn decrypt_retrieve_data(&self, data: Bytes) -> BytesMut {
        data.into()
    }
}

impl DownloadFunctions<String> for LowMemoryDownloadFunctions {
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
}

impl FilenSDK {
    pub async fn attempt_download_chunk_task<T>(
        link: FsURL,
        i: u64,
        download_funcs: &impl DownloadFunctions<T>,
    ) -> Result<T, FilenSDKError>
    where
        T: Send + Sync + 'static,
    {
        let mut tries = 0;
        while tries < DOWNLOAD_RETRIES {
            let link = link.clone();

            let result: Result<T, FilenSDKError> = download_funcs.http_retrieve_data(link, i).await;
            match result {
                Ok(data) => {
                    return Ok(data);
                }
                Err(e) => {
                    eprintln!("Error downloading chunk {}: {}. Retrying...", i, e);

                    // Wait 1 second before retrying
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    tries += 1;

                    if tries >= DOWNLOAD_RETRIES {
                        eprintln!("Failed to download chunk {}", i);
                        // tx_decrypt.send((i, None)).await.unwrap();
                        return Err(FilenSDKError::DownloadError {
                            err_str: format!("Failed to download chunk {}", i),
                        });
                    }
                }
            };
        }

        Err(FilenSDKError::DownloadError {
            err_str: format!("Failed to download chunk {}", i),
        })
    }

    pub async fn decrypt_chunk_task<T>(
        i: u64,
        data: T,
        key: &[u8],
        download_funcs: &impl DownloadFunctions<T>,
    ) -> Result<BytesMut, FilenSDKError> {
        let mut data = download_funcs.decrypt_retrieve_data(data);

        if data.len() > 0 {
            let decrypt_in_memory =
                crate::crypto::file_decrypt::decrypt_v2_bytes(&mut data, key).unwrap();

            return Ok(decrypt_in_memory);
        } else {
            return Err(FilenSDKError::DownloadError {
                err_str: format!("Error downloading chunk {}, empty message", i),
            });
        }
    }
}

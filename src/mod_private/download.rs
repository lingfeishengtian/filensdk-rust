use std::{future::Future, sync::Arc, time::Duration};

use bytes::{Bytes, BytesMut};

use super::{net_interaction::FilenNetInteractionFunctions, DOWNLOAD_RETRIES};
use crate::{error::FilenSDKError, httpclient::{download_into_memory, download_to_file_streamed, FsURL}, FilenSDK};

impl FilenSDK {
    pub async fn attempt_download_chunk_task<T>(
        link: FsURL,
        i: u64,
        download_funcs: &impl FilenNetInteractionFunctions<T>,
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
        download_funcs: &impl FilenNetInteractionFunctions<T>,
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

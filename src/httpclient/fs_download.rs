use std::{future::Future, sync::atomic::AtomicI8, time::Duration};

use bytes::{Bytes, BytesMut};
use tokio::runtime::{EnterGuard, Handle, Runtime};

use super::FsURL;
use crate::{
    crypto::file_decrypt::{write_output, write_output_async}, error::FilenSDKError, filensdk::MAX_DOWNLOAD_THREADS,
    FilenSDK,
};

pub const DOWNLOAD_RETRIES: u64 = 3;

impl FilenSDK {
    // fn desugared_decrypt_thread<T>(key: String, output_file_path: std::path::PathBuf, output_name: Option<String>, start_chunk: u64, mut rx_decrypt: tokio::sync::mpsc::Receiver<(u64, Option<T>)>, decrypt_retrieve_data: impl Fn(T) -> Bytes + Send + Sync + 'static)
    // -> impl Future<Output = AtomicI8> + Send + Sync
    // where T: Send + Sync + 'static,
    // {
    //     async move {
    //         while let Some((i, data)) = rx_decrypt.recv().await {
    //             if data.is_none() {
    //                 eprintln!("Error downloading chunk {}, empty message", i);
    //             }

    //             let data = decrypt_retrieve_data(data.unwrap());

    //             if data.len() > 0 {
    //                 let mut data = data.try_into_mut().unwrap();
    //                 let decrypt_in_memory = crate::crypto::file_decrypt::decrypt_v2_in_memory(
    //                     &mut data,
    //                     key.as_bytes(),
    //                 )
    //                 .unwrap();

    //                 if let Some(output_name) = &output_name {
    //                     let out_path = output_file_path.join(output_name);
    //                     write_output(&out_path.as_path(), decrypt_in_memory, Some((i - start_chunk) as usize));
    //                 } else {
    //                     let out_path = output_file_path.join(format!("{}", i));
    //                     write_output(&out_path, decrypt_in_memory, None);
    //                 }
    //             }
    //         }

    //         AtomicI8::new(0)
    //     }
    // }

    // Self is not needed since client is not explicitly used
    pub async fn download_file_generic<T, B>(
        &self,
        uuid: &str,
        region: &str,
        bucket: &str,
        key: String,
        output_dir: &std::path::Path,
        output_name: Option<String>,
        file_size: u64,
        http_retrieve_data: impl Fn(FsURL, u64) -> B + Send + Sync + Clone + 'static,
        decrypt_retrieve_data: impl (Fn(T) -> Bytes) + Send + Sync + 'static,
        byte_range_start: u64,
        byte_range_end: u64,
    ) -> Result<(u64, u64), FilenSDKError>
    where
        T: Send + Sync + 'static,
        B: std::future::Future<Output = Result<T, FilenSDKError>> + Sync + Send + 'static,
    {
        // Create output directory if it does not exist
        std::fs::create_dir_all(output_dir)?;

        if let Some(output_name) = &output_name {
            let out_path = output_dir.join(output_name);
            if out_path.is_dir() {
                return Err(FilenSDKError::PathIsDirectory {
                    path: output_name.clone(),
                });
            }
        }

        // // Create tokio runtime using builder to configure multi-threaded runtime
        // let rt = tokio::runtime::Builder::new_multi_thread()
        //     .enable_all()
        //     .build()
        //     .unwrap();
        // let _guard = rt.enter();

        // let rt_handle = rt.handle();

        // Start channel for finished tasks to notify completion
        let (tx_decrypt, mut rx_decrypt) =
            tokio::sync::mpsc::channel::<(u64, Option<T>)>(MAX_DOWNLOAD_THREADS);

        // Calculate start and end chunk range
        let start_chunk = std::cmp::max(byte_range_start / crate::crypto::CHUNK_SIZE as u64, 0);
        let end_chunk = std::cmp::min(
            (byte_range_end + crate::crypto::CHUNK_SIZE as u64 - 1)
                / crate::crypto::CHUNK_SIZE as u64,
            file_size / crate::crypto::CHUNK_SIZE as u64 + 1,
        );

        let output_file_path = output_dir.to_path_buf();
        println!("Downloading chunks {} to {}", start_chunk, end_chunk);

        let mut started_threads = 0;

        // Thread spawner thread

        let semaphore = self.download_semaphore.clone();

        let uuid = uuid.to_string();
        let region = region.to_string();
        let bucket = bucket.to_string();

        tokio::spawn(async move {
            // Start download threads
            for i in start_chunk..end_chunk {
                let semaphore_reserve = semaphore.clone();
                let permit = semaphore_reserve.acquire_owned().await;
                let http_retrieve_data = http_retrieve_data.clone();

                let tx_decrypt = tx_decrypt.clone();

                started_threads += 1;

                // // Semaphore to limit number of concurrent downloads

                let link = crate::httpclient::FsURL::Egest(
                    region.to_string(),
                    bucket.to_string(),
                    uuid.to_string(),
                    i,
                );

                tokio::spawn(async move {
                    let mut tries = 0;
                    while tries < DOWNLOAD_RETRIES {
                        let link = link.clone();

                        let result: Result<T, FilenSDKError> = http_retrieve_data(link, i).await;
                        match result {
                            Ok(data) => {
                                // println!("Downloaded chunk {}", i);
                                tx_decrypt.send((i, Some(data))).await.unwrap();

                                break;
                            }
                            Err(e) => {
                                // TODO: Retry logic
                                eprintln!("Error downloading chunk {}: {}. Retrying...", i, e);

                                // Wait 1 second before retrying
                                tokio::time::sleep(Duration::from_secs(1)).await;
                                tries += 1;

                                if tries >= DOWNLOAD_RETRIES {
                                    eprintln!("Failed to download chunk {}", i);
                                    tx_decrypt.send((i, None)).await.unwrap();
                                }
                            }
                        };
                    }
                });
            }
        });

        // let _ = handle.block_on(handler);
        while let Some((i, data)) = rx_decrypt.recv().await {
            if data.is_none() {
                eprintln!("Error downloading chunk {}, empty message", i);

                return Err(FilenSDKError::DownloadError {
                    err_str: format!(
                        "Error downloading chunk {}, empty message",
                        i,
                    ),
                });
            }

            let data = decrypt_retrieve_data(data.unwrap());

            if data.len() > 0 {
                let mut data = data.try_into_mut().unwrap();
                let decrypt_in_memory =
                    crate::crypto::file_decrypt::decrypt_v2_in_memory(&mut data, key.as_bytes())
                        .unwrap();

                if let Some(output_name) = &output_name {
                    let out_path = output_file_path.join(output_name);
                    write_output_async(
                        &out_path,
                        decrypt_in_memory,
                        Some((i - start_chunk) as usize),
                    ).await;
                } else {
                    let out_path = output_file_path.join(format!("{}", i));
                    write_output_async(&out_path, decrypt_in_memory, None).await;
                }
            }
        }

        Ok((
            start_chunk * crate::crypto::CHUNK_SIZE as u64,
            std::cmp::min(end_chunk * crate::crypto::CHUNK_SIZE as u64, file_size),
        ))
    }
}

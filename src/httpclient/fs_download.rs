use std::time::Duration;

use bytes::{Bytes, BytesMut};

use super::FsURL;
use crate::{
    crypto::file_decrypt::write_output, error::FilenSDKError, filensdk::MAX_DOWNLOAD_THREADS,
    FilenSDK,
};

impl FilenSDK {
    // Self is not needed since client is not explicitly used
    pub async fn download_file_generic<T, G, B>(
        &self,
        uuid: &str,
        region: &str,
        bucket: &str,
        key: String,
        output_dir: &std::path::Path,
        output_name: Option<String>,
        file_size: u64,
        http_retrieve_data: impl Fn(FsURL, u64) -> B + Send + Sync + Clone + 'static,
        decrypt_retrieve_data: G,
        byte_range_start: u64,
        byte_range_end: u64,
    ) -> Result<(u64, u64), FilenSDKError>
    where
        T: Send + 'static,
        B: std::future::Future<Output = Result<T, FilenSDKError>> + Send + 'static,
        G: (Fn(T) -> Bytes) + Send + 'static,
    {
        // Create output directory if it does not exist
        std::fs::create_dir_all(output_dir)?;

        if let Some(output_name) = &output_name {
            let out_path = output_dir.join(output_name);
            if out_path.is_dir() {
                return Err(FilenSDKError::PathIsDirectory { path: output_name.clone() });
            }
        }

        // Create tokio runtime using builder to configure multi-threaded runtime
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        let _guard = rt.enter();

        // Start channel for finished tasks to notify completion
        let (tx, mut rx) = tokio::sync::mpsc::channel::<(u64, bool)>(MAX_DOWNLOAD_THREADS);
        let (tx_decrypt, mut rx_decrypt) =
            tokio::sync::mpsc::channel::<(u64, Option<T>)>(MAX_DOWNLOAD_THREADS);

        // Calculate start and end chunk range
        let start_chunk = byte_range_start / crate::crypto::CHUNK_SIZE as u64;
        let end_chunk = (byte_range_end + crate::crypto::CHUNK_SIZE as u64 - 1) / crate::crypto::CHUNK_SIZE as u64;
        
        let output_file_path = output_dir.to_path_buf();
        let handler = tokio::task::spawn_blocking(move || {
            while let Some((i, data)) = rx_decrypt.blocking_recv() {
                if data.is_none() {
                    eprintln!("Error downloading chunk {}, empty message", i);
                }

                let data = decrypt_retrieve_data(data.unwrap());

                if data.len() > 0 {
                    let mut data = data.try_into_mut().unwrap();
                    let decrypt_in_memory = crate::crypto::file_decrypt::decrypt_v2_in_memory(
                        &mut data,
                        key.as_bytes(),
                    )
                    .unwrap();

                    if let Some(output_name) = &output_name {
                        let out_path = output_file_path.join(output_name);
                        write_output(&out_path.as_path(), decrypt_in_memory, Some((i - start_chunk) as usize));
                    } else {
                        let out_path = output_file_path.join(format!("{}", i));
                        write_output(&out_path, decrypt_in_memory, None);
                    }
                }
            }
        });

        println!("Downloading chunks {} to {}", start_chunk, end_chunk);

        let mut started_threads = 0;

        // Start download threads
        for i in start_chunk..end_chunk {
            // Block to conserve stack
            if started_threads > MAX_DOWNLOAD_THREADS as u64 {
                let prev_res = rx.blocking_recv().unwrap();

                crate::return_function_on_result_fail!(prev_res);
            }

            started_threads += 1;

            // // Semaphore to limit number of concurrent downloads
            let semaphore = self.download_semaphore.clone();

            let tx = tx.clone();
            let tx_decrypt = tx_decrypt.clone();

            let http_retrieve_data = http_retrieve_data.clone();
            let link = crate::httpclient::FsURL::Egest(
                region.to_string(),
                bucket.to_string(),
                uuid.to_string(),
                i,
            );

            tokio::spawn(async move {
                // Obtain permit in thread
                println!("Recieved chunk {}", i);
                let _permit: Result<tokio::sync::SemaphorePermit<'_>, tokio::sync::AcquireError> =
                    semaphore.acquire().await;
                let link = link.clone();

                let result: Result<T, FilenSDKError> = http_retrieve_data(link, i).await;
                match result {
                    Ok(data) => {
                        println!("Downloaded chunk {}", i);
                        tx_decrypt.send((i, Some(data))).await.unwrap();
                        tx.send((i, true)).await.unwrap();
                    }
                    Err(e) => {
                        // TODO: Retry logic
                        eprintln!("Error downloading chunk {}: {}", i, e);

                        // Wait 1 second before retrying
                        tokio::time::sleep(Duration::from_secs(1)).await;

                        tx.send((i, false)).await.unwrap();
                    }
                };
            });
        }

        drop(tx);
        drop(tx_decrypt);

        // Clear tx
        while let Some(resp) = rx.blocking_recv() {
            crate::return_function_on_result_fail!(resp);
        }

        let _ = rt.block_on(handler);

        Ok((
            start_chunk * crate::crypto::CHUNK_SIZE as u64,
            std::cmp::min(end_chunk * crate::crypto::CHUNK_SIZE as u64, file_size)
        ))
    }
}

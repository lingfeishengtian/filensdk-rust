use std::{
    collections::VecDeque,
    future::Future,
    sync::{atomic::AtomicI8, Arc},
    time::Duration,
};

use bytes::{Bytes, BytesMut};
use futures::Stream;
use tokio::{
    runtime::{EnterGuard, Handle, Runtime},
    task::JoinHandle,
};

use super::FsURL;
use crate::{
    crypto::file_decrypt::{decrypt_v2_bytes, write_output, write_output_async},
    error::FilenSDKError,
    filensdk::{MAX_DOWNLOAD_THREADS, MAX_READ_AHEAD_THREADS},
    mod_private::download::{DownloadFunctions, LowDiskDownloadFunctions},
    FilenSDK, CHUNK_SIZE,
};

impl FilenSDK {
    /// This method of download does not care about the order of the chunks, and will download them in parallel.
    /// This is useful for downloading large files, when streaming is not necessary.
    pub async fn orderless_file_download<T>(
        &self,
        uuid: &str,
        region: &str,
        bucket: &str,
        key: String,
        output_dir: &std::path::Path,
        output_name: Option<String>,
        file_size: u64,
        byte_range_start: u64,
        byte_range_end: u64,
        download_funcs: impl DownloadFunctions<T>,
    ) -> Result<(u64, u64), FilenSDKError>
    where
        T: Send + Sync + 'static,
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

        let semaphore = self.download_semaphore.clone();

        let uuid = uuid.to_string();
        let region = region.to_string();
        let bucket = bucket.to_string();

        let cloned_download_funcs = download_funcs.clone();
        tokio::spawn(async move {
            // Start download threads
            for i in start_chunk..end_chunk {
                let semaphore_reserve = semaphore.clone();
                let permit = semaphore_reserve.acquire_owned().await;
                let tx_decrypt = tx_decrypt.clone();

                let link = crate::httpclient::FsURL::Egest(
                    region.to_string(),
                    bucket.to_string(),
                    uuid.to_string(),
                    i,
                );

                let cloned_download_funcs = cloned_download_funcs.clone();

                tokio::spawn(async move {
                    let _moved_permit = permit;
                    let result = Self::attempt_download_chunk_task(link, i, &cloned_download_funcs)
                        .await
                        .ok();
                    tx_decrypt.send((i, result)).await.unwrap();
                });
            }
        });

        while let Some((i, data)) = rx_decrypt.recv().await {
            if data.is_none() {
                eprintln!("Error downloading chunk {}, empty message", i);

                return Err(FilenSDKError::DownloadError {
                    err_str: format!("Error downloading chunk {}, empty message", i,),
                });
            }

            let decrypt_in_memory =
                Self::decrypt_chunk_task(i, data.unwrap(), &key.as_bytes(), &download_funcs)
                    .await?;

            if let Some(output_name) = &output_name {
                let out_path = output_file_path.join(output_name);
                write_output_async(
                    &out_path,
                    &decrypt_in_memory,
                    Some((i - start_chunk) as usize),
                )
                .await?;
            } else {
                let out_path = output_file_path.join(format!("{}", i));
                write_output_async(&out_path, &decrypt_in_memory, None).await?;
            }
        }

        Ok((
            start_chunk * crate::crypto::CHUNK_SIZE as u64,
            std::cmp::min(end_chunk * crate::crypto::CHUNK_SIZE as u64, file_size),
        ))
    }

    async fn summon_single_download_decrypt_task(
        i: u64,
        link: FsURL,
        client: Arc<reqwest::Client>,
        key: String,
    ) -> Option<Bytes> {
        let download_method = LowDiskDownloadFunctions {
            client: client.clone(),
        };
        let downloaded_bytes = FilenSDK::attempt_download_chunk_task(link, i, &download_method)
            .await
            .ok();
        match downloaded_bytes {
            Some(bytes) => {
                let mut prepared_bytes_to_decrypt = download_method.decrypt_retrieve_data(bytes);
                let decrypted_bytes =
                    decrypt_v2_bytes(&mut prepared_bytes_to_decrypt, key.as_bytes());

                match decrypted_bytes {
                    Ok(bytes) => Some(bytes.freeze()),
                    Err(e) => {
                        eprintln!("Error decrypting chunk: {:?}", e);
                        None
                    }
                }
            }
            None => None,
        }
    }

    // TODO: Allow for custom download functions
    /// Stream downloaded chunks, this method is sensitive to the order of the chunks and will not continue until the previous chunk is downloaded.
    /// However, it will look ahead and download the next MAX_READ_AHEAD_THREADS chunks in parallel.
    pub fn read_ahead_download_stream(
        &self,
        size: u64,
        start_byte: u64,
        region: &str,
        bucket: &str,
        uuid: &str,
        key: String,
    ) -> impl Stream<Item = Result<Bytes, FilenSDKError>> {
        let region = region.to_string();
        let bucket = bucket.to_string();
        let uuid = uuid.to_string();

        let total_chunks = size / (CHUNK_SIZE as u64) + 1;
        let start_chunk = start_byte / (CHUNK_SIZE as u64);

        let client = self.client.clone();
        async_stream::stream! {
            let mut current_chunk = start_chunk;
            let mut task_deque: VecDeque<JoinHandle<Option<Bytes>>> = VecDeque::new();

            let top_chunk = std::cmp::min(start_chunk + MAX_READ_AHEAD_THREADS + 1, total_chunks);
            for i in start_chunk..top_chunk {
                println!("Starting chunk {} for start byte {} and size {}", i, start_byte, size);
                let link = crate::httpclient::FsURL::Egest(
                    region.to_string(),
                    bucket.to_string(),
                    uuid.to_string(),
                    i,
                );

                task_deque.push_back(tokio::spawn(Self::summon_single_download_decrypt_task(i, link, client.clone(), key.clone())));
            }

            loop {
                if current_chunk >= total_chunks {
                    break;
                }

                let result = task_deque.pop_front();
                let data = result.unwrap().await.unwrap();
                if data.is_none() {
                    eprintln!("Error downloading chunk, empty message");

                    break;
                }

                // If first chunk, then start from offset of start_byte
                let start_offset = if current_chunk == start_chunk {
                    let start_offset = start_byte % (CHUNK_SIZE as u64);
                    println!("Starting from offset {}", start_offset);
                    start_offset
                } else {
                    0
                };
                // yield Ok(hyper::body::Frame::data(data.unwrap().slice(start_offset as usize..)));
                yield Ok(data.unwrap().slice(start_offset as usize..));


                current_chunk += 1;
                if current_chunk < total_chunks {
                    let link = crate::httpclient::FsURL::Egest(
                        region.to_string(),
                        bucket.to_string(),
                        uuid.to_string(),
                        current_chunk + MAX_READ_AHEAD_THREADS,
                    );

                    task_deque.push_back(tokio::spawn(Self::summon_single_download_decrypt_task(current_chunk + MAX_READ_AHEAD_THREADS, link, client.clone(), key.clone())));
                }
            }
        }
    }
}

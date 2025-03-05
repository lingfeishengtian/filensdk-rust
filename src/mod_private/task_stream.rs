use std::{cmp::min, collections::VecDeque, convert::Infallible, ops::RangeInclusive, sync::Arc, time::Duration};

use bytes::Bytes;
use futures_core::Stream;
use range_set::RangeSet;
use tokio::task::JoinHandle;

use crate::{
    crypto::file_decrypt::decrypt_v2_in_memory,
    error::FilenSDKError,
    httpclient::{download_into_memory, fs_download::DOWNLOAD_RETRIES, FsURL}, CHUNK_SIZE,
};

async fn summon_download_task(
    index: u64,
    url: FsURL,
    client: Arc<reqwest::Client>,
    key: String,
) -> Option<Bytes> {
    let mut tries = 0;
    let key_bytes = key.as_bytes();

    let mut bytes: Option<Bytes> = None;

    while tries < DOWNLOAD_RETRIES {
        let result = download_into_memory(&url, &client).await;
        match result {
            Ok(data) => {
                // Decrypt
                let mut mutable_data = data.try_into_mut().unwrap();
                let dec = decrypt_v2_in_memory(&mut mutable_data, key_bytes);

                if dec.is_err() {
                    tries += 1;
                    continue;
                }

                bytes = Some(Bytes::copy_from_slice(dec.unwrap()));
                break;
            }
            Err(e) => {
                // TODO: Retry logic
                eprintln!("Error downloading chunk: {}. Retrying...", e);

                // Wait 1 second before retrying
                tokio::time::sleep(Duration::from_secs(1)).await;
                tries += 1;

                if tries >= DOWNLOAD_RETRIES {
                    eprintln!("Failed to download chunk");
                }
            }
        };
    }

    bytes
}

const MAX_READ_AHEAD_THREADS: u64 = 50;
pub fn read_ahead_download_stream(
    size: u64,
    start_byte: u64,
    client: Arc<reqwest::Client>,
    region: &str,
    bucket: &str,
    uuid: &str,
    key: String,
) -> impl Stream<Item = Result<hyper::body::Frame<Bytes>, Infallible>> {
    let region = region.to_string();
    let bucket = bucket.to_string();
    let uuid = uuid.to_string();

    let total_chunks = size / (CHUNK_SIZE as u64) + 1;
    let start_chunk = start_byte / (CHUNK_SIZE as u64);
    async_stream::stream! {
        let mut current_chunk = start_chunk;
        let mut task_deque: VecDeque<JoinHandle<Option<Bytes>>> = VecDeque::new();

        let top_chunk = min(start_chunk + MAX_READ_AHEAD_THREADS + 1, total_chunks);
        for i in start_chunk..top_chunk {
            println!("Starting chunk {} for start byte {} and size {}", i, start_byte, size);
            let link = crate::httpclient::FsURL::Egest(
                region.to_string(),
                bucket.to_string(),
                uuid.to_string(),
                i,
            );

            task_deque.push_back(tokio::spawn(summon_download_task(i, link, client.clone(), key.clone())));
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
            yield Ok(hyper::body::Frame::data(data.unwrap().slice(start_offset as usize..)));

            current_chunk += 1;
            if current_chunk < total_chunks {
                let link = crate::httpclient::FsURL::Egest(
                    region.to_string(),
                    bucket.to_string(),
                    uuid.to_string(),
                    current_chunk + MAX_READ_AHEAD_THREADS,
                );

                task_deque.push_back(tokio::spawn(summon_download_task(current_chunk + MAX_READ_AHEAD_THREADS, link, client.clone(), key.clone())));
            }
        }
    }
}

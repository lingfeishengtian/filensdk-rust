use std::{sync::Arc, time::Duration};

use bytes::Bytes;

use crate::{crypto::file_decrypt::write_output, error::FilenSDKError, filensdk::MAX_DOWNLOAD_THREADS, FilenSDK};
use super::FsURL;

pub async fn download_file<T, G, B>(
    download_semaphore: Arc<tokio::sync::Semaphore>,
    uuid: &str,
    region: &str,
    bucket: &str,
    key: String,
    output_dir: &str,
    file_name: &str,
    file_size: u64,
    http_retrieve_data: impl Fn(FsURL, u64) -> B + Send + Sync + Clone + 'static,
    decrypt_retrieve_data: G,
) where T: Send + 'static,
    B: std::future::Future<Output = Result<T, FilenSDKError>> + Send + 'static,
    G: (Fn(T) -> Bytes) + Send + 'static
{
    // Calculate chunks round up
    let chunks = (file_size as f64 / crate::crypto::CHUNK_SIZE as f64).ceil() as u64;

    // Does output directory exist?
    if !std::path::Path::new(&output_dir).exists() {
        std::fs::create_dir(&output_dir).unwrap();
    }

    let file_path = format!("{}/{}", output_dir, file_name);

    // Create tokio runtime using builder to configure multi-threaded runtime
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let _guard = rt.enter();

    // Start channel for finished tasks to notify completion
    let (tx, mut rx) = tokio::sync::mpsc::channel::<u64>(MAX_DOWNLOAD_THREADS);
    let (tx_decrypt, mut rx_decrypt) = tokio::sync::mpsc::channel::<(u64, Option<T>)>(32);

    let file_path_clone = file_path.clone();
    let handler = tokio::task::spawn_blocking(move || {
        while let Some((i, data)) = rx_decrypt.blocking_recv() {
            if data.is_none() {
                eprintln!("Error downloading chunk {}, empty message", i);
            }
            
            let data = decrypt_retrieve_data(data.unwrap());

            if data.len() > 0 {
                let mut data = data.try_into_mut().unwrap();
                let decrypt_in_memory = crate::crypto::file_decrypt::decrypt_v2_in_memory(&mut data, key.as_bytes()).unwrap();
                let out_path = std::path::Path::new(&file_path_clone).to_path_buf();
                write_output(&out_path.as_path(), decrypt_in_memory, Some(i as usize));
            }
        }
    });


    // Start download threads
    for i in 0..chunks {
        // Block to conserve stack
        if i > MAX_DOWNLOAD_THREADS as u64 {
            rx.blocking_recv().unwrap();
        }

        // // Semaphore to limit number of concurrent downloads
        let semaphore = download_semaphore.clone();

        let tx = tx.clone();
        let tx_decrypt = tx_decrypt.clone();
        
        let http_retrieve_data = http_retrieve_data.clone();
        let link = crate::httpclient::FsURL::Egest(region.to_string(), bucket.to_string(), uuid.to_string(), i);

        tokio::spawn(async move {
            // Obtain permit in thread
            let _permit: Result<tokio::sync::SemaphorePermit<'_>, tokio::sync::AcquireError> = semaphore.acquire().await;
            let result: Result<T, FilenSDKError> = http_retrieve_data(link, i).await;
            match result {
                Ok(data) => {
                    tx_decrypt.send((i, Some(data))).await.unwrap();
                    tx.send(i).await.unwrap();
                    drop(tx_decrypt);
                }
                Err(e) => {
                    // TODO: Retry logic
                    eprintln!("Error downloading chunk {}: {}", i, e);
                    tx_decrypt.send((i, None)).await.unwrap();
                    tx.send(i).await.unwrap();
                    drop(tx_decrypt);
                }
            };
        });
    }

    drop(tx);
    drop(tx_decrypt);

    // Clear tx
    while let Some(_) = rx.blocking_recv() {}

    let _ = rt.block_on(handler);
}
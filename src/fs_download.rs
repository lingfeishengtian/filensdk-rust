use std::{sync::Arc, time::Duration};

use bytes::Bytes;

use crate::{crypto::file_decrypt::write_output, error::FilenSDKError, filensdk::MAX_DOWNLOAD_THREADS, httpclient::{download_into_memory, FilenURL}, FilenSDK};


async fn download_file<T, G, B>(
    download_semaphore: Arc<tokio::sync::Semaphore>,
    uuid: String,
    region: String,
    bucket: String,
    key: String,
    output_dir: String,
    file_name: String,
    file_size: u64,
    // http_retrieve_data: F,
    http_retrieve_data: impl Fn(FilenURL) -> B + Send + Clone + 'static,
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
                println!("Error downloading chunk {}, empty message", i);
            }
            
            let data = decrypt_retrieve_data(data.unwrap());

            if data.len() > 0 {
                println!("Decrypting chunk {} to {}", i, file_path_clone);
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
        
        // Copy link enum
        let link = crate::httpclient::FilenURL::egest(region.to_string(), bucket.to_string(), uuid.to_string(), i);
        // let key = key.to_string();

        println!("settup for chunk {}", i);
        let http_retrieve_data = http_retrieve_data.clone();

        // Start download thread
        tokio::spawn(async move {
            // Copy permit into thread
            let _permit: Result<tokio::sync::SemaphorePermit<'_>, tokio::sync::AcquireError> = semaphore.acquire().await;

            let cur_time = std::time::SystemTime::now();
            // let result = crate::httpclient::download_into_memory(link, &client).await;
            let result: Result<T, FilenSDKError> = http_retrieve_data(link).await;
            match result {
                Ok(data) => {
                    println!("Downloaded chunk {} in {} ms", i, cur_time.elapsed().unwrap().as_millis());
                    
                    tx_decrypt.send((i, Some(data))).await.unwrap();
                    drop(tx_decrypt);
                    tx.send(i).await.unwrap();
                }
                Err(e) => {
                    // TODO: Retry logic
                    eprintln!("Error downloading chunk {}: {}", i, e);
                    tx_decrypt.send((i, None)).await.unwrap();
                    drop(tx_decrypt);
                    tx.send(i).await.unwrap();
                }
            };
        });
    }

    drop(tx);
    drop(tx_decrypt);

    let _ = rt.block_on(handler);
}

#[uniffi::export]
impl FilenSDK {
    async fn download_file_low_disk(
        &self,
        uuid: String,
        region: String,
        bucket: String,
        key: String,
        output_dir: String,
        file_name: String,
        file_size: u64,
    ) {
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
        let (tx, mut rx) = tokio::sync::mpsc::channel(MAX_DOWNLOAD_THREADS);
        let (tx_decrypt, mut rx_decrypt) = tokio::sync::mpsc::channel::<(u64, bytes::Bytes)>(32);

        let file_path_clone = file_path.clone();
        let handler = tokio::task::spawn_blocking(move || {
            while let Some((i, data)) = rx_decrypt.blocking_recv() {
                if data.len() > 0 {
                    println!("Decrypting chunk {} to {}", i, file_path_clone);
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
            let semaphore = self.download_semaphore.clone();

            let tx = tx.clone();
            let tx_decrypt = tx_decrypt.clone();
            
            // Copy link enum
            let link = crate::httpclient::FilenURL::egest(region.to_string(), bucket.to_string(), uuid.to_string(), i);
            // let key = key.to_string();
            let client = self.client.clone();

            println!("settup for chunk {}", i);
            // Start download thread
            tokio::spawn(async move {
                // Copy permit into thread
                let _permit: Result<tokio::sync::SemaphorePermit<'_>, tokio::sync::AcquireError> = semaphore.acquire().await;

                let cur_time = std::time::SystemTime::now();
                let result = crate::httpclient::download_into_memory(link, &client).await;
                match result {
                    Ok(data) => {
                        println!("Downloaded chunk {} in {} ms", i, cur_time.elapsed().unwrap().as_millis());
                        
                        tx_decrypt.send((i, data)).await.unwrap();
                        drop(tx_decrypt);
                        tx.send(i).await.unwrap();
                    }
                    Err(e) => {
                        // TODO: Retry logic
                        eprintln!("Error downloading chunk {}: {}", i, e);
                        tx_decrypt.send((i, vec![].into())).await.unwrap();
                        drop(tx_decrypt);
                        tx.send(i).await.unwrap();
                    }
                }
            });
        }

        drop(tx);
        drop(tx_decrypt);

        let _ = rt.block_on(handler);
    }

    /*
    For scenarios where memory is extremely strained, use streaming and file writing to avoid using
    more memory. However, this may be slower than the in-memory decryption method.
    */
    async fn download_file_low_memory(
        &self,
        uuid: String,
        region: String,
        bucket: String,
        key: String,
        output_dir: String,
        tmp_dir: String,
        file_name: String,
        file_size: u64,
    ) {
        
        // Calculate chunks round up
        let chunks = (file_size as f64 / crate::crypto::CHUNK_SIZE as f64).ceil() as u64;

        // Does output directory exist?
        if !std::path::Path::new(&output_dir).exists() {
            std::fs::create_dir(&output_dir).unwrap();
        }

        if !std::path::Path::new(&tmp_dir).exists() {
            std::fs::create_dir(&tmp_dir).unwrap();
        }

        let out_path = format!("{}/{}", output_dir, file_name);

        // Create tokio runtime using builder to configure multi-threaded runtime
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        let _guard = rt.enter();

        // Start channel for finished tasks to notify completion
        let (tx, mut rx) = tokio::sync::mpsc::channel(MAX_DOWNLOAD_THREADS);
        let (tx_decrypt, mut rx_decrypt) = tokio::sync::mpsc::channel(32);

        let handler = tokio::task::spawn_blocking(move || {
            while let Some((i, file_path)) = rx_decrypt.blocking_recv() {
                if file_path != "" {
                    println!("Decrypting chunk {} to {}", i, file_path);
                    let file_path = std::path::Path::new(&file_path).to_path_buf();
                    let key = key.clone();
                    let mut data = std::fs::read(&file_path).unwrap();
                    let decrypt_in_memory = crate::crypto::file_decrypt::decrypt_v2_in_memory(data.as_mut_slice(), key.as_bytes()).unwrap();
                    
                    let out_path = std::path::Path::new(&out_path).to_path_buf();
                    write_output(&out_path.as_path(), decrypt_in_memory, Some(i as usize));

                    // Delete tmp file
                    std::fs::remove_file(file_path).unwrap();
                }
            }
        });

        // Start download threads
        for i in 0..chunks {
            // Block to conserve stack
            if i > MAX_DOWNLOAD_THREADS as u64 {
                rt.block_on(rx.recv()).unwrap();
            }

            // // Semaphore to limit number of concurrent downloads
            let semaphore = self.download_semaphore.clone();

            let tx = tx.clone();
            let tx_decrypt = tx_decrypt.clone();
            
            // Copy link enum
            let link = crate::httpclient::FilenURL::egest(region.to_string(), bucket.to_string(), uuid.to_string(), i);
            let file_path = tmp_dir.clone() + &format!("/chunk_{}", i);
            let client = self.client.clone();

            // Start download thread
            tokio::spawn(async move {
                // Copy permit into thread
                let _permit: Result<tokio::sync::SemaphorePermit<'_>, tokio::sync::AcquireError> = semaphore.acquire().await;

                let result = crate::httpclient::download_to_file_streamed(link, &client, &file_path).await;
                match result {
                    Ok(data) => {
                        tx.send(i).await.unwrap();
                        tx_decrypt.send((i, file_path)).await.unwrap();
                        drop(tx_decrypt);
                    }
                    Err(e) => {
                        // TODO: Retry logic
                        eprintln!("Error downloading chunk {}: {}", i, e);
                        tx.send(i).await.unwrap();
                        tx_decrypt.send((i, "".to_string())).await.unwrap();
                        drop(tx_decrypt);
                    }
                }
            });
        }

        drop(tx);
        drop(tx_decrypt);

       let _ = rt.block_on(handler);

        // Clear tmp dir
        std::fs::remove_dir_all(tmp_dir).unwrap();

        // Wait for all downloads to finish
        // let chunks_to_wait = if chunks > MAX_DOWNLOAD_THREADS as u64 {
        //     MAX_DOWNLOAD_THREADS as u64
        // } else {
        //     chunks
        // };
        // for _ in 0..chunks_to_wait {
        //     rt.block_on(rx.recv()).unwrap();
        // }
        // rt.block_on(rx.recv()).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::remove_file;
    use std::fs::File;
    use std::io::Write;

    #[async_std::test]
    async fn test_download_file() {
        dotenv::dotenv().ok();
        let sdk = FilenSDK::new();

        // Import credentials from dotenv
        let creds = std::env::var("TEST_CRED_IMPORT").unwrap();
        sdk.import_credentials(creds);

        let uuid = std::env::var("TEST_UUID").unwrap();
        let region = std::env::var("TEST_REGION").unwrap();
        let bucket = std::env::var("TEST_BUCKET").unwrap();
        let key = std::env::var("TEST_KEY").unwrap();
        let output_dir = std::env::var("TEST_OUTPUT_DIR").unwrap();
        let file_name = std::env::var("TEST_FILE_NAME").unwrap();
        let file_size: u64 = std::env::var("TEST_FILE_SIZE").unwrap().parse().unwrap();


        let current_time = std::time::SystemTime::now();
        sdk.download_file_low_disk(uuid, region, bucket, key, output_dir.clone(), file_name.clone(), file_size).await;
        // sdk.download_file_low_memory(uuid, region, bucket, key, output_dir.clone(), "tests/tmp".to_string(), file_name.clone(), file_size).await;
        let elapsed = current_time.elapsed().unwrap();
        println!("Download speed: {} MB/s", file_size as f64 / elapsed.as_secs_f64() / 1024.0 / 1024.0);

        // Compare sha256 of downloaded file with original using ring::digest
        let sha = std::env::var("TEST_FILE_SHA256").unwrap();
        let file_path = format!("{}/{}", output_dir, file_name);
        let file = File::open(file_path).unwrap();
        let mut reader = std::io::BufReader::new(file);
        let mut context = ring::digest::Context::new(&ring::digest::SHA256);
        let mut buffer = [0; 1024];
        loop {
            let count = std::io::Read::read(&mut reader, &mut buffer).unwrap();
            if count == 0 {
                break;
            }
            context.update(&buffer[..count]);
        }
        let digest = context.finish();
        let digest = digest.as_ref();
        let digest = hex::encode(digest);
        assert_eq!(digest, sha);
    }
}
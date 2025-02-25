use std::sync::Arc;

use crate::{crypto::file_decrypt::write_output, error::FilenSDKError, filensdk::MAX_DOWNLOAD_THREADS, FilenSDK};

#[uniffi::export]
impl FilenSDK {
    async fn download_file(
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
            .worker_threads(4)
            .enable_all()
            .build()
            .unwrap();
        let _guard = rt.enter();

        // Start channel for finished tasks to notify completion
        let (tx, mut rx) = tokio::sync::mpsc::channel(32);

        // Start download threads
        for i in 0..chunks {
            // Block to conserve stack
            if i > MAX_DOWNLOAD_THREADS as u64 {
                rt.block_on(rx.recv()).unwrap();
            }

            // // Semaphore to limit number of concurrent downloads
            let semaphore = self.download_semaphore.clone();

            let tx = tx.clone();
            
            // Copy link enum
            let link = crate::httpclient::FilenURL::egest(region.to_string(), bucket.to_string(), uuid.to_string(), i);
            let file_path = file_path.clone();
            let file_path = std::path::Path::new(&file_path).to_path_buf();
            let key = key.to_string();
            let client = self.client.clone();

            // Start download thread
            tokio::spawn(async move {
                // Copy permit into thread
                let _permit: Result<tokio::sync::SemaphorePermit<'_>, tokio::sync::AcquireError> = semaphore.acquire().await;

                let result = crate::httpclient::download_into_memory(link, &client).await;
                match result {
                    Ok(data) => {
                        let mut data = data;
                        let decrypt_in_memory = crate::crypto::file_decrypt::decrypt_v2_in_memory(&mut data, key.as_bytes()).unwrap();
                        write_output(file_path.as_path(), decrypt_in_memory, Some(i as usize));

                        tx.send(i).await.unwrap();
                    }
                    Err(e) => {
                        // TODO: Retry logic
                        eprintln!("Error downloading chunk {}: {}", i, e);
                        tx.send(i).await.unwrap();
                    }
                }
            });
        }

        // Wait for all downloads to finish
        let chunks_to_wait = if chunks > MAX_DOWNLOAD_THREADS as u64 {
            MAX_DOWNLOAD_THREADS as u64
        } else {
            chunks
        };
        for _ in 0..chunks_to_wait {
            rt.block_on(rx.recv()).unwrap();
        }
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
        sdk.download_file(uuid, region, bucket, key, output_dir.clone(), file_name.clone(), file_size).await;
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
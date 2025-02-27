use bytes::Bytes;

use crate::{httpclient::{download_into_memory, download_to_file_streamed, fs_download::download_file, FsURL}, FilenSDK};

#[uniffi::export]
impl FilenSDK {
    pub async fn download_file_low_disk(
        &self,
        uuid: String,
        region: String,
        bucket: String,
        key: String,
        output_dir: String,
        file_name: String,
        file_size: u64,
    ) {
        let client = self.client.clone();
        download_file(
            self.download_semaphore.clone(),
            &uuid,
            &region,
            &bucket,
            key,
            &output_dir,
            &file_name,
            file_size,
            move |url: FsURL, _index: u64| {
                let client = client.clone();
                async move {
                    download_into_memory(url, &client).await
                }
            },
            |data: Bytes| -> Bytes { data }
        ).await;
    }

    /*
    For scenarios where memory is extremely strained, use streaming and file writing to avoid using
    more memory. However, this may be slower than the in-memory decryption method.
    */
    pub async fn download_file_low_memory(
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
        let client = self.client.clone();

        // Create tmp directory if it doesn't exist
        if !std::path::Path::new(&tmp_dir).exists() {
            std::fs::create_dir(&tmp_dir).unwrap();
        }

        download_file(
            self.download_semaphore.clone(),
            &uuid,
            &region,
            &bucket,
            key,
            &output_dir,
            &file_name,
            file_size,
            move |url: FsURL, index: u64| {
                let client = client.clone();
                let tmp_dir = tmp_dir.clone() + "/" + &index.to_string();
                async move {
                    download_to_file_streamed(url, &client, &tmp_dir).await
                }
            },
            |data: String| -> Bytes { 
                let file = std::fs::File::open(&data).unwrap();
                let mut reader = std::io::BufReader::new(file);
                let mut buffer = Vec::new();
                std::io::Read::read_to_end(&mut reader, &mut buffer).unwrap();

                // Delete the file after reading
                std::fs::remove_file(&data).unwrap();

                Bytes::from(buffer)
             }
        ).await;
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
        // sdk.download_file_low_disk(uuid, region, bucket, key, output_dir.clone(), file_name.clone(), file_size).await;
        sdk.download_file_low_memory(uuid, region, bucket, key, output_dir.clone(), "tests/tmp".to_string(), file_name.clone(), file_size).await;
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
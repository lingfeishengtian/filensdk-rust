// Create one thread for encrypting

// main thread waits for finished encrypted data

// create another thread for uploading which is limited by the upload semaphore and MAX UPLOAD THREADS

use std::io::Read;

use bytes::Bytes;

use crate::{httpclient::{self, httpclient::{upload_from_file, upload_from_memory}}, FilenSDK};

#[uniffi::export]
impl FilenSDK {
    /// Uploads a file to the filen service, automatically handling encryption and threading the upload
    /// process. Optimized for scenarios where memory is not a concern. Rather than writing uploaded chunks
    /// to a separate file, the chunks are stored in memory and encrypted in memory. At a maximum, MAX_UPLOAD_THREADS
    /// * 2 * CHUNK_SIZE amount of memory will be used (more depending on how malloc functions on the system).
    /// 
    /// # Arguments
    /// 
    /// * `input_file` - The path to the file to upload
    /// * `filen_parent` - The parent folder to upload the file to (uuid)
    /// * `name` - What to name the file on the filen service
    /// * `should_use_counter_nonce` - Whether to use a counter nonce or a random nonce for encryption
    /// 
    /// # Returns
    /// 
    /// The uuid of the uploaded file
    /// 
    /// # Extra Info
    /// 
    /// The nonce used for encryption is generated using the `generate_counter_iv` and `generate_rand_iv` functions.
    /// There are security concerns with using a random nonce since repeated nonces can leak information about the
    /// plaintext. generate_rand_inv has an extremely low chance of generating the same nonce twice, but it is still
    /// possible. The counter nonce is a safer option since it is guaranteed to be unique for every chunk. The only
    /// concern is that all files with have the same nonce for the same chunk index.
    pub async fn upload_file_low_disk(
        &self,
        input_file: String,
        filen_parent: String,
        name: String,
        should_use_counter_nonce: bool,
    ) -> Result<String, crate::error::FilenSDKError> {
        let client = self.client.clone();
        let api_key = self.api_key()?;
        let input_file = input_file.to_string();

        self.upload_file_generic(
            &input_file.clone(),
            &filen_parent,
            &name,
            move |i, key| {
                let input_file = input_file.clone();
                let (encrypted_data, hash) = crate::crypto::file_encrypt::encrypt_v2_from_file(
                    &input_file,
                    None,
                    key,
                    i as usize,
                    should_use_counter_nonce
                )
                .unwrap();
                (encrypted_data, hash)
            },
            move |url, data| {
                let client = client.clone();
                {
                    let value = api_key.clone();
                    async move { upload_from_memory(url, &client, data.unwrap(), &value).await }
                }
            },
        )
        .await
    }


    /// Uploads a file to the filen service, automatically handling encryption and threading the upload
    /// process. Optimized for scenarios where memory is a concern. Rather than storing chunks to memory,
    /// the chunks are written to disk and encrypted in memory. Only one encrypt thread is ran at a time,
    /// but multiple upload threads can be ran at once. However, upload threads use streamed uploads, so
    /// the memory usage is minimal. At a maximum, CHUNK_SIZE + MAX_UPLOAD_THREADS * STREAM_MEMORY amount
    /// of memory will be used. **NOTE: This function causes disk usage to double due to the temporary files
    /// created for encryption.**
    /// 
    /// # Arguments
    /// 
    /// * `input_file` - The path to the file to upload
    /// * `filen_parent` - The parent folder to upload the file to (uuid)
    /// * `name` - What to name the file on the filen service
    /// * `tmp_output_dir` - The directory to store the temporary files used for encryption
    /// * `should_use_counter_nonce` - Whether to use a counter nonce or a random nonce for encryption
    /// 
    /// # Returns
    /// 
    /// The uuid of the uploaded file
    /// 
    /// # Extra Info
    /// 
    /// The nonce used for encryption is generated using the `generate_counter_iv` and `generate_rand_iv` functions.
    /// There are security concerns with using a random nonce since repeated nonces can leak information about the
    /// plaintext. generate_rand_inv has an extremely low chance of generating the same nonce twice, but it is still
    /// possible. The counter nonce is a safer option since it is guaranteed to be unique for every chunk. The only
    /// concern is that all files with have the same nonce for the same chunk index.
    pub async fn upload_file_low_memory(
        &self,
        input_file: String,
        filen_parent: String,
        name: String,
        tmp_output_dir: String,
        should_use_counter_nonce: bool,
    ) -> Result<String, crate::error::FilenSDKError> {
        let client = self.client.clone();
        let api_key = self.api_key()?;
        let input_file = input_file.to_string();

        // Create tmp directory if it doesn't exist
        if !std::path::Path::new(&tmp_output_dir).exists() {
            std::fs::create_dir(&tmp_output_dir).unwrap();
        }

        self.upload_file_generic(
            &input_file.clone(),
            &filen_parent,
            &name,
            move |i, key| {
                let input_file = input_file.clone();
                // let nonce = if should_use_counter_nonce {
                //     crate::crypto::generate_counter_iv(i)
                // } else {
                //     crate::crypto::generate_rand_iv().unwrap()
                // };
                let output_file = tmp_output_dir.to_string() + "/" + &i.to_string();
                let (_encrypted_data, hash) = crate::crypto::file_encrypt::encrypt_v2_from_file(
                    &input_file,
                    Some(&output_file),
                    key,
                    i as usize,
                    should_use_counter_nonce
                )
                .unwrap();
                (output_file, hash)
            },
            move |url, data| {
                let client = client.clone();
                {
                    let value = api_key.clone();
                    async move { 
                        let fut = upload_from_file(url, &client, &data, &value).await;
                        std::fs::remove_file(&data).unwrap();
                        fut
                    }
                }
            },
        )
        .await
    }

    /*
    Convience functions
    */

    /// Calls `upload_file_low_disk` with `should_use_counter_nonce` set to false which aligns with
    /// the default behavior of the filen service's own SDK.
    pub async fn upload_file_low_disk_default(
        &self,
        input_file: String,
        filen_parent: String,
        name: String,
    ) -> Result<String, crate::error::FilenSDKError> {
        self.upload_file_low_disk(input_file, filen_parent, name, false).await
    }

    /// Calls `upload_file_low_memory` with `should_use_counter_nonce` set to false which aligns with
    /// the default behavior of the filen service's own SDK.
    pub async fn upload_file_low_memory_default(
        &self,
        input_file: String,
        filen_parent: String,
        name: String,
        tmp_output_dir: String,
    ) -> Result<String, crate::error::FilenSDKError> {
        self.upload_file_low_memory(input_file, filen_parent, name, tmp_output_dir, false).await
    }

    /// Calls `upload_file_low_disk` with `should_use_counter_nonce` set to false. Since applications
    /// where low memory is a concern are less common, the default behavior is set to use more memory
    /// while conserving disk usage. In a normal desktop/mobile enviornment, every single file upload
    /// should take around 200 MB of memory.
    pub async fn upload_file(
        &self,
        input_file: String,
        filen_parent: String,
        name: String,
    ) -> Result<String, crate::error::FilenSDKError> {
        self.upload_file_low_disk(input_file, filen_parent, name, false).await
    }
}

#[cfg(test)]
mod tests {
    use crate::requests::fs::FileMetadata;

    use super::*;

    #[async_std::test]
    async fn test_upload_file() {
        // let input_file = "tests/out/test.txt";
        let input_file = "tests/out/Pixelmon-1.16.5-9.1.12-ARM-Mac-FIxed.jar";
        let filensdk = crate::filensdk::FilenSDK::new();

        dotenv::dotenv().ok();
        filensdk.import_credentials(dotenv::var("TEST_CRED_IMPORT").unwrap());

        let filen_parent = filensdk.base_folder().unwrap();
        // generate random file name
        let name = uuid::Uuid::new_v4().to_string() + ".txt";

        let result = filensdk
            // .upload_file_low_disk(input_file.to_string(), filen_parent, name, true)
            .upload_file_low_memory(input_file.to_string(), filen_parent, name, "tests/tmp/test_up".to_string(), true)
            .await;
        assert!(result.is_ok());

        let uuid = result.unwrap();

        // Download file
        let download_path = "tests/out/test_download_out";
        let file_info = filensdk.file_info(uuid.clone()).await.unwrap();

        let decrypted_metadata = crate::crypto::metadata::decrypt_metadata(
            &file_info.metadata.as_bytes(),
            &filensdk.master_key().unwrap(),
        )
        .unwrap();
        let decrypted_metadata_str = String::from_utf8(decrypted_metadata).unwrap();
        let metadata: FileMetadata = serde_json::from_str(&decrypted_metadata_str).unwrap();

        let download_result = filensdk
            .download_file(uuid, download_path.to_string())
            .await;

        // Compare files
        let file = std::fs::File::open(input_file).unwrap();
        let mut reader = std::io::BufReader::new(file);
        let mut buffer = Vec::new();
        std::io::Read::read_to_end(&mut reader, &mut buffer).unwrap();

        let file = std::fs::File::open(download_path.to_owned() + "/" + &metadata.name.to_owned())
            .unwrap();
        let mut reader = std::io::BufReader::new(file);
        let mut buffer_download = Vec::new();
        std::io::Read::read_to_end(&mut reader, &mut buffer_download).unwrap();

        assert_eq!(buffer, buffer_download);
    }
}

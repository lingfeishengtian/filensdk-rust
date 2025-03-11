// Create one thread for encrypting

// main thread waits for finished encrypted data

// create another thread for uploading which is limited by the upload semaphore and MAX UPLOAD THREADS

use std::io::Read;

use bytes::Bytes;
use uniffi_shared_tokio_runtime_proc::uniffi_async_export;

use crate::{httpclient::{self, httpclient::{upload_from_file, upload_from_memory}}, mod_private::net_interaction::{LowDiskInteractionFunctions, LowMemoryInteractionFunctions}, FilenSDK};


#[uniffi_async_export]
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
            LowDiskInteractionFunctions {
                client: client.clone(),
                api_key: api_key.clone(),
                should_use_counter_nonce,
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
            LowMemoryInteractionFunctions {
                client: client.clone(),
                api_key: api_key.clone(),
                tmp_dir: tmp_output_dir.clone(),
                should_use_counter_nonce,
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
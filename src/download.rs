use bytes::Bytes;
use uniffi_shared_tokio_runtime_proc::uniffi_async_export;

use crate::{
    file::FilenFileDetailed,
    mod_private::net_interaction::{LowDiskInteractionFunctions, LowMemoryInteractionFunctions},
    FilenSDK,
};

#[derive(uniffi::Record)]
pub struct FileByteRange {
    pub start_byte: u64,
    pub end_byte: u64,
}

#[derive(uniffi::Record)]
pub struct FilenFileDownloadResult {
    pub file_byte_range: FileByteRange,
    pub file_info: FilenFileDetailed,
}

macro_rules! extract_path_and_filename {
    ($output_file:expr) => {{
        let file_path = std::path::Path::new(&$output_file);

        let output_dir = match file_path.parent() {
            Some(parent) => match parent.to_str() {
                Some(parent_str) => parent_str.to_string(),
                None => {
                    return Err(crate::error::FilenSDKError::InvalidPath { path: $output_file })
                }
            },
            None => return Err(crate::error::FilenSDKError::InvalidPath { path: $output_file }),
        };

        let file_name = match file_path.file_name() {
            Some(name) => match name.to_str() {
                Some(name_str) => name_str.to_string(),
                None => {
                    return Err(crate::error::FilenSDKError::InvalidPath { path: $output_file })
                }
            },
            None => return Err(crate::error::FilenSDKError::InvalidPath { path: $output_file }),
        };

        (output_dir, file_name)
    }};
}

#[uniffi_async_export]
impl FilenSDK {
    /// Intentionally shared function for cases where all information is known, or more a greater
    /// need for control over the download process is needed.
    pub async fn internal_download_file_low_disk(
        &self,
        uuid: String,
        region: String,
        bucket: String,
        key: String,
        output_dir: String,
        output_filename: Option<String>,
        file_size: u64,
        start_byte: Option<u64>,
        end_byte: Option<u64>,
    ) -> Result<FileByteRange, crate::error::FilenSDKError> {
        let client = self.client.clone();

        let start_byte = start_byte.unwrap_or(0);
        let end_byte = end_byte.unwrap_or(file_size);

        let output_dir = std::path::Path::new(&output_dir);

        self.orderless_file_download(
            &uuid,
            &region,
            &bucket,
            key,
            output_dir,
            output_filename,
            file_size,
            start_byte,
            end_byte,
            LowDiskInteractionFunctions {
                client: client.clone(),
                api_key: "".to_string(),
                should_use_counter_nonce: false,
            },
        )
        .await
        .map(|downloaded_range| FileByteRange {
            start_byte: downloaded_range.0,
            end_byte: downloaded_range.1,
        })
    }

    /// Intentionally shared function for cases where all information is known, or more a greater
    /// need for control over the download process is needed.
    ///
    /// For more information with the parameters to this function, see the documentation for Filen's API.
    pub async fn internal_download_file_low_memory(
        &self,
        uuid: String,
        region: String,
        bucket: String,
        key: String,
        output_dir: String,
        output_filename: Option<String>,
        tmp_dir: String,
        file_size: u64,
        start_byte: Option<u64>,
        end_byte: Option<u64>,
    ) -> Result<FileByteRange, crate::error::FilenSDKError> {
        let client = self.client.clone();

        // Create tmp directory if it doesn't exist
        if !std::path::Path::new(&tmp_dir).exists() {
            std::fs::create_dir(&tmp_dir).unwrap();
        }

        let start_byte = start_byte.unwrap_or(0);
        let end_byte = end_byte.unwrap_or(file_size);

        let output_dir = std::path::Path::new(&output_dir);

        self.orderless_file_download(
            &uuid,
            &region,
            &bucket,
            key,
            &output_dir,
            output_filename,
            file_size,
            start_byte,
            end_byte,
            LowMemoryInteractionFunctions {
                client: client.clone(),
                api_key: "".to_string(),
                tmp_dir,
                should_use_counter_nonce: false,
            },
        )
        .await
        .map(|downloaded_range| FileByteRange {
            start_byte: downloaded_range.0,
            end_byte: downloaded_range.1,
        })
    }

    /*
    Convenience functions
    */

    /// Convenience function to download a partial file into memory.
    pub async fn download_partial_file(
        &self,
        uuid: String,
        output_file: String,
        start_byte: Option<u64>,
        end_byte: Option<u64>,
    ) -> Result<FilenFileDownloadResult, crate::error::FilenSDKError> {
        // Retrieve and decrypt metadata
        let metadata = self.file_info(uuid.clone()).await?;
        let (output_dir, file_name) = extract_path_and_filename!(output_file);

        Ok(FilenFileDownloadResult {
            file_info: metadata.clone(),
            file_byte_range: self
                .internal_download_file_low_disk(
                    metadata.uuid,
                    metadata.region,
                    metadata.bucket,
                    String::from_utf8(metadata.key).unwrap(),
                    output_dir,
                    Some(file_name),
                    metadata.size,
                    start_byte,
                    end_byte,
                )
                .await?,
        })
    }

    /// Convenience function to download a partial file with low memory usage.
    pub async fn download_partial_file_low_memory(
        &self,
        uuid: String,
        output_file: String,
        tmp_dir: String,
        start_byte: Option<u64>,
        end_byte: Option<u64>,
    ) -> Result<FilenFileDownloadResult, crate::error::FilenSDKError> {
        // Retrieve and decrypt metadata
        let metadata = self.file_info(uuid.clone()).await?;
        let (output_dir, file_name) = extract_path_and_filename!(output_file);

        // Download the partial file
        Ok(FilenFileDownloadResult {
            file_info: metadata.clone(),
            file_byte_range: self
                .internal_download_file_low_memory(
                    metadata.uuid,
                    metadata.region,
                    metadata.bucket,
                    String::from_utf8(metadata.key).unwrap(),
                    output_dir,
                    Some(file_name),
                    tmp_dir,
                    metadata.size,
                    start_byte,
                    end_byte,
                )
                .await?,
        })
    }

    /// Download the file to the specified output_dir all in chunks. The output will have a folder
    /// with a bunch of files that are the chunks of the original file. A chunk is CHUNK_SIZE bytes
    /// and the files will be titled with their chunk index.
    pub async fn download_file_chunked(
        &self,
        uuid: String,
        output_dir: String,
        start_byte: Option<u64>,
        end_byte: Option<u64>,
    ) -> Result<FilenFileDownloadResult, crate::error::FilenSDKError> {
        // Retrieve and decrypt metadata
        let metadata = self.file_info(uuid.clone()).await?;

        Ok(FilenFileDownloadResult {
            file_info: metadata.clone(),
            file_byte_range: self
                .internal_download_file_low_disk(
                    metadata.uuid,
                    metadata.region,
                    metadata.bucket,
                    String::from_utf8(metadata.key).unwrap(),
                    output_dir,
                    None,
                    metadata.size,
                    start_byte,
                    end_byte,
                )
                .await?,
        })
    }

    /// See download_file_chunked for more information. This function is for scenarios where memory
    /// is extremely strained.
    pub async fn download_file_chunked_low_memory(
        &self,
        uuid: String,
        output_dir: String,
        tmp_dir: String,
        start_byte: Option<u64>,
        end_byte: Option<u64>,
    ) -> Result<FilenFileDownloadResult, crate::error::FilenSDKError> {
        // Retrieve and decrypt metadata
        let metadata = self.file_info(uuid.clone()).await?;

        Ok(FilenFileDownloadResult {
            file_info: metadata.clone(),
            file_byte_range: self
                .internal_download_file_low_memory(
                    metadata.uuid,
                    metadata.region,
                    metadata.bucket,
                    String::from_utf8(metadata.key).unwrap(),
                    output_dir,
                    None,
                    tmp_dir,
                    metadata.size,
                    start_byte,
                    end_byte,
                )
                .await?,
        })
    }

    /// For scenarios when memory is not a concern, use this function to download the file into memory
    /// and write it to the output file.
    ///
    /// # Arguments
    /// - `uuid` - The UUID of the file to download.
    /// - `output_file` - The path to the file to write the downloaded file to.
    ///
    /// # Returns
    /// A `FileByteRange` struct with the start and end byte of the downloaded file.
    pub async fn download_file(
        &self,
        uuid: String,
        output_file: String,
    ) -> Result<FilenFileDownloadResult, crate::error::FilenSDKError> {
        self.download_partial_file(uuid, output_file, None, None)
            .await
    }

    /// For scenarios where memory is extremely strained, use streaming and file writing to avoid using
    /// more memory. However, this may be slower than the in-memory decryption method.
    pub async fn download_file_low_memory(
        &self,
        uuid: String,
        output_file: String,
        tmp_dir: String,
    ) -> Result<FilenFileDownloadResult, crate::error::FilenSDKError> {
        self.download_partial_file_low_memory(uuid, output_file, tmp_dir, None, None)
            .await
    }
}

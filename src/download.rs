use bytes::Bytes;
use uniffi_shared_tokio_runtime_proc::uniffi_async_export;

use crate::{
    httpclient::{download_into_memory, download_to_file_streamed, FsURL}, mod_private::download::{LowDiskDownloadFunctions, LowMemoryDownloadFunctions}, FilenSDK
};

#[derive(uniffi::Record)]
pub struct FileByteRange {
    pub start_byte: u64,
    pub end_byte: u64,
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
            LowDiskDownloadFunctions { client: client.clone() },
        )
        .await
        .map(|downloaded_range| FileByteRange {
            start_byte: downloaded_range.0,
            end_byte: downloaded_range.1,
        })
    }

    // /// Intentionally shared function for cases where all information is known, or more a greater
    // /// need for control over the download process is needed.
    // ///
    // /// For more information with the parameters to this function, see the documentation for Filen's API.
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
            LowMemoryDownloadFunctions {
                client: client.clone(),
                tmp_dir,
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
    ) -> Result<FileByteRange, crate::error::FilenSDKError> {
        // Retrieve and decrypt metadata
        let metadata = self.file_info(uuid.clone()).await?;
        let decrypted = self.decrypt_metadata(metadata.metadata, self.master_key()?)?;

        let (output_dir, file_name) = extract_path_and_filename!(output_file);

        // Download the partial file
        self.internal_download_file_low_disk(
            metadata.uuid,
            metadata.region,
            metadata.bucket,
            String::from_utf8(decrypted.key).unwrap(),
            output_dir,
            Some(file_name),
            decrypted.size.unwrap_or(0),
            start_byte,
            end_byte,
        ).await
    }

    /// Convenience function to download a partial file with low memory usage.
    pub async fn download_partial_file_low_memory(
        &self,
        uuid: String,
        output_file: String,
        tmp_dir: String,
        start_byte: Option<u64>,
        end_byte: Option<u64>,
    ) -> Result<FileByteRange, crate::error::FilenSDKError> {
        // Retrieve and decrypt metadata
        let metadata = self.file_info(uuid.clone()).await?;
        let decrypted = self.decrypt_metadata(metadata.metadata, self.master_key()?)?;

        let (output_dir, file_name) = extract_path_and_filename!(output_file);

        // Download the partial file
        self.internal_download_file_low_memory(
            metadata.uuid,
            metadata.region,
            metadata.bucket,
            String::from_utf8(decrypted.key).unwrap(),
            output_dir,
            Some(file_name),
            tmp_dir,
            decrypted.size.unwrap_or(0),
            start_byte,
            end_byte,
        ).await
    }

    /// Download the file to the specified output_dir all in chunks. The output will have a folder
    /// with a bunch of files that are the chunks of the original file. A chunk is CHUNK_SIZE bytes
    /// and the files will be titled with their chunk index.
    pub async fn download_file_chunked(
        &self,
        uuid: String,
        output_dir: String,
        start_byte: Option<u64>,
        end_byte: Option<u64>
    ) -> Result<FileByteRange, crate::error::FilenSDKError> {
        // Retrieve and decrypt metadata
        let metadata = self.file_info(uuid.clone()).await?;
        let decrypted = self.decrypt_metadata(metadata.metadata, self.master_key()?)?;

        println!("download_file_chunked: metadata:");
        self.internal_download_file_low_disk(
            metadata.uuid,
            metadata.region,
            metadata.bucket,
            String::from_utf8(decrypted.key).unwrap(),
            output_dir,
            None,
            decrypted.size.unwrap_or(0),
            start_byte,
            end_byte,
        ).await
    }

    /// See download_file_chunked for more information. This function is for scenarios where memory
    /// is extremely strained.
    pub async fn download_file_chunked_low_memory(
        &self,
        uuid: String,
        output_dir: String,
        tmp_dir: String,
        start_byte: Option<u64>,
        end_byte: Option<u64>
    ) -> Result<FileByteRange, crate::error::FilenSDKError> {
        // Retrieve and decrypt metadata
        let metadata = self.file_info(uuid.clone()).await?;
        let decrypted = self.decrypt_metadata(metadata.metadata, self.master_key()?)?;

        self.internal_download_file_low_memory(
            metadata.uuid,
            metadata.region,
            metadata.bucket,
            String::from_utf8(decrypted.key).unwrap(),
            output_dir,
            None,
            tmp_dir,
            decrypted.size.unwrap_or(0),
            start_byte,
            end_byte,
        ).await
    }

    /// For scenarios when memory is not a concern, use this function to download the file into memory.
    pub async fn download_file(
        &self,
        uuid: String,
        output_file: String,
    ) -> Result<(), crate::error::FilenSDKError> {
        // Download the file
        self.download_partial_file(uuid, output_file, None, None).await?;

        Ok(())
    }

    /// For scenarios where memory is extremely strained, use streaming and file writing to avoid using
    /// more memory. However, this may be slower than the in-memory decryption method.
    pub async fn download_file_low_memory(
        &self,
        uuid: String,
        output_file: String,
        tmp_dir: String,
    ) -> Result<(), crate::error::FilenSDKError> {
        // Download the file
        self.download_partial_file_low_memory(uuid, output_file, tmp_dir, None, None).await?;

        Ok(())
    }
}

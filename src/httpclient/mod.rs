mod endpoints;
pub mod fs_download;
pub mod fs_upload;
pub mod httpclient;

pub use endpoints::Endpoints;
pub use endpoints::FsURL;
pub use httpclient::{download_into_memory, download_to_file_streamed, http_none, make_request};

use crate::FilenSDK;

#[macro_export]
macro_rules! return_function_on_result_fail {
    ($prev_res:expr) => {
        if !$prev_res.1 {
            eprintln!("Error at chunk {}", $prev_res.0);

            return Err(FilenSDKError::DownloadError {
                err_str: format!(
                    "Error at chunk {}, timeout or connection reset, stopping whole download.",
                    $prev_res.0
                ),
            })?;
        }
    };
}

pub fn calculate_chunk_range(start_byte: u64, end_byte: u64, file_size: u64) -> (u64, u64) {
    let start_chunk = std::cmp::max(start_byte / crate::crypto::CHUNK_SIZE as u64, 0);
    let end_chunk = std::cmp::min(
        (end_byte + crate::crypto::CHUNK_SIZE as u64 - 1) / crate::crypto::CHUNK_SIZE as u64,
        file_size / crate::crypto::CHUNK_SIZE as u64 + 1,
    );

    (start_chunk, end_chunk)
}

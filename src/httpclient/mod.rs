pub mod httpclient;
pub mod fs_download;
pub mod fs_upload;
mod endpoints;

pub use httpclient::{http_none, make_request, download_into_memory, download_to_file_streamed};
pub use endpoints::FsURL;
pub use endpoints::Endpoints;

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
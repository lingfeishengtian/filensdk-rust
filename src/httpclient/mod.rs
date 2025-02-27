pub mod httpclient;
pub mod fs_download;
pub mod fs_upload;
mod endpoints;

pub use httpclient::{http_none, make_request, download_into_memory, download_to_file_streamed};
pub use endpoints::FsURL;
pub use endpoints::Endpoints;
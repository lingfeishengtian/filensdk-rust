uniffi::setup_scaffolding!();

pub mod credentials;
pub mod filensdk;
pub mod download_stream;
pub mod auth;
pub mod responses;
pub mod error;
pub mod dir;

pub mod upload;
pub mod download;
pub mod file;

pub mod httpserver;
// pub mod upload;

mod httpclient;
mod crypto;
mod requests;
mod mod_private;

pub use filensdk::FilenSDK;
pub use crypto::CHUNK_SIZE;
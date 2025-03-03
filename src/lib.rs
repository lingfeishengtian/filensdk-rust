uniffi::setup_scaffolding!();

pub mod credentials;
pub mod filensdk;
pub mod auth;
pub mod responses;
pub mod error;

pub mod upload;
pub mod download;
pub mod file;
// pub mod upload;

mod httpclient;
mod crypto;
mod requests;
mod mod_private;

pub use filensdk::FilenSDK;
pub use crypto::CHUNK_SIZE;
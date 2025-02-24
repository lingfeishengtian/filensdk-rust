uniffi::setup_scaffolding!();

pub mod credentials;
pub mod filensdk;
pub mod auth;
pub mod responses;
pub mod error;

mod httpclient;
mod crypto;
mod requests;

pub use filensdk::FilenSDK;
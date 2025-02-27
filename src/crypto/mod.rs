use std::io;

use ring::rand::{SecureRandom, SystemRandom};

pub mod password;
pub mod file_decrypt;
pub mod file_encrypt;
pub mod metadata;

pub const CHUNK_SIZE: usize = 1024 * 1024;

#[derive(Debug)]
pub enum CryptoError {
    Io(io::Error),
    Ring(ring::error::Unspecified),
    InvalidMetadata,
}

impl From<io::Error> for CryptoError {
    fn from(err: io::Error) -> Self {
        CryptoError::Io(err)
    }
}

impl From<ring::error::Unspecified> for CryptoError {
    fn from(err: ring::error::Unspecified) -> Self {
        CryptoError::Ring(err)
    }
}

impl std::fmt::Display for CryptoError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            CryptoError::Io(err) => write!(f, "IO Error: {}", err),
            CryptoError::Ring(err) => write!(f, "Ring Error: {}", err),
            CryptoError::InvalidMetadata => write!(f, "Invalid Metadata"),
        }
    }
}

/*
For compatability with the other Filen SDKs, we can only use alphanumeric characters in the encryption key.
*/
fn generate_alphanumeric_random_string(buffer: &mut [u8]) -> Result<(), CryptoError> {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let rng = SystemRandom::new();
    rng.fill(buffer)?;
    for i in buffer.iter_mut() {
        *i = CHARSET[*i as usize % CHARSET.len()];
    }
    Ok(())
}

pub fn generate_rand_key() -> Result<[u8; 32], CryptoError> {
    let mut key = [0u8; 32];
    generate_alphanumeric_random_string(&mut key)?;
    Ok(key)
}

pub fn generate_rand_iv() -> Result<[u8; 12], CryptoError> {
    let mut iv = [0u8; 12];
    generate_alphanumeric_random_string(&mut iv)?;
    Ok(iv)
}
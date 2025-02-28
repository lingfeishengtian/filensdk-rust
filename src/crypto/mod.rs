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

const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

/*
For compatability with the other Filen SDKs, we can only use alphanumeric characters in the encryption key.
*/
fn generate_alphanumeric_random_string(buffer: &mut [u8]) -> Result<(), CryptoError> {
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

pub fn generate_counter_iv(counter: u64) -> [u8; 12] {
    let mut buffer = [b'A'; 12]; // Initialize buffer with 'A's
    let base = CHARSET.len() as u64;

    let mut counter = counter;

    for i in 0..12 {
        let remainder = (counter % base) as usize;
        buffer[11 - i] = CHARSET[remainder];
        counter /= base;
    }

    buffer
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_rand_key() {
        let key = generate_rand_key().unwrap();
        assert_eq!(key.len(), 32);
    }

    #[test]
    fn test_generate_rand_iv() {
        let iv = generate_rand_iv().unwrap();
        assert_eq!(iv.len(), 12);
    }

    #[test]
    fn test_generate_counter_iv() {
        let iv = generate_counter_iv(0);
        assert_eq!(iv, [b'A'; 12]);

        let iv = generate_counter_iv(1);
        assert_eq!(&iv, b"AAAAAAAAAAAB");

        let iv = generate_counter_iv(63);
        assert_eq!(&iv, b"AAAAAAAAAABB");
    }
}
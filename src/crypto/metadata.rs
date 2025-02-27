use std::num::NonZero;

use base64::{prelude::{BASE64_STANDARD, BASE64_STANDARD_NO_PAD}, Engine};
use ring::{digest::{digest, SHA1_FOR_LEGACY_USE_ONLY, SHA512}, rand::{SecureRandom, SystemRandom}};

use super::{generate_rand_iv, CryptoError};

pub fn hash_fn(message: &str) -> Result<String, CryptoError> {
    let sha512_digest = digest(&SHA512, message.as_bytes());
    let sha1_digest = digest(&SHA1_FOR_LEGACY_USE_ONLY, sha512_digest.as_ref());

    Ok(hex::encode(sha1_digest.as_ref()))
}

fn transform_key(key: &str) -> [u8; 32] {
    let mut transformed_key: [u8; 32] = [0; 32];
    ring::pbkdf2::derive(
        ring::pbkdf2::PBKDF2_HMAC_SHA512,
        NonZero::new(1).unwrap(),
        key.as_bytes(),
        key.as_bytes(),
        &mut transformed_key
    );

    transformed_key
}

pub fn encrypt_metadata(str: &[u8], key: &str) -> Result<Vec<u8>, CryptoError> {
    let transformed_key: [u8; 32] = transform_key(key);

    let iv = generate_rand_iv()?;

    let sealing_key = ring::aead::LessSafeKey::new(
        ring::aead::UnboundKey::new(&ring::aead::AES_256_GCM, &transformed_key)?
    );

    let nonce = ring::aead::Nonce::assume_unique_for_key(iv);
    let associated_data = ring::aead::Aad::empty();

    let mut data = str.to_vec();
    sealing_key.seal_in_place_append_tag(nonce, associated_data, &mut data)?;
    
    let fin = ["002".as_bytes(), &iv, BASE64_STANDARD.encode(&data).as_bytes()].concat();
    Ok(fin)
}

pub fn decrypt_metadata(str: &[u8], key: &str) -> Result<Vec<u8>, CryptoError> {
    if str.len() < 12 + ring::aead::AES_256_GCM.tag_len() {
        return Err(CryptoError::InvalidMetadata);
    }

    // Ensure first 3 characters are "002"
    if &str[0..3] != "002".as_bytes() {
        return Err(CryptoError::InvalidMetadata);
    }

    // Only allocate new memory for the data portion (This should be the only time we allocate memory for O(n) data)
    let str = str[3..].to_vec();

    // O(1) copy for iv
    let iv: [u8; 12] = str[0..12].try_into().unwrap();
    let mut data = BASE64_STANDARD.decode(&str[12..]).unwrap();

    let transformed_key: [u8; 32] = transform_key(key);

    let opening_key = ring::aead::LessSafeKey::new(
        ring::aead::UnboundKey::new(&ring::aead::AES_256_GCM, &transformed_key)?
    );

    let nonce = ring::aead::Nonce::assume_unique_for_key(iv);
    let associated_data = ring::aead::Aad::empty();

    let data = opening_key.open_in_place(nonce, associated_data, &mut data)?;

    Ok(data.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decrypt_metadata() {
        let data = "002GIAtrOwdWqdelZba7dSXKFEG0mZ6JmWvYLtt0HDkGxFQyPYqSvA=";
        let key = "abcdabcdabcdabcdabcdabcdabcdabcd";
        let decrypted = decrypt_metadata(data.as_bytes(), key).unwrap();
        assert_eq!(String::from_utf8(decrypted).unwrap(), "Test Metadata");
    }

    #[test]
    fn test_encrypt_metadata() {
        let data = "Test Metadata";
        let key = "abcdabcdabcdabcdabcdabcdabcdabcd";
        let encrypted = encrypt_metadata(data.as_bytes(), key).unwrap();

        // Decrypt the encrypted data
        let decrypted = decrypt_metadata(&encrypted, key).unwrap();
        assert_eq!(String::from_utf8(decrypted).unwrap(), data);
    }
}
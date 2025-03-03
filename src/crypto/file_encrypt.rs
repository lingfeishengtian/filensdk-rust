use std::{
    error::Error,
    io::{self, Read, Seek, Write},
};

use ring::{
    aead::{self, chacha20_poly1305_openssh::TAG_LEN, Tag}, digest::{digest, SHA512}, rand::{self, SecureRandom}
};

use super::{generate_counter_iv, CryptoError, CHUNK_SIZE};

/*
Optimized encrypt function that prevents copying too much within memory.
This function reads data from a file and encrypts it in place and then writes it to an output file
if it exists. Otherwise, it returns the encrypted data.
*/
pub fn encrypt_v2_from_file(
    input: &str,
    output: Option<&str>,
    key_bytes: &[u8; 32],
    index: usize,
    should_use_counter_nonce: bool,
) -> Result<(Option<Vec<u8>>, String), CryptoError> {
    // Does file exist
    if !std::path::Path::new(input).exists() {
        return Err(CryptoError::Io(io::Error::new(
            io::ErrorKind::NotFound,
            "Input file not found",
        )));
    }

    // Open file and verify index
    let mut input_file = std::fs::File::open(input)?;
    let metadata = input_file.metadata()?;
    let file_size = metadata.len().try_into().unwrap();

    if index * CHUNK_SIZE > file_size {
        return Err(CryptoError::Io(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Index out of bounds",
        )));
    }

    let size_of_chunk = if (index + 1) * CHUNK_SIZE > file_size {
        file_size % CHUNK_SIZE
    } else {
        CHUNK_SIZE
    };

    let mut data: Vec<u8> = vec![0; (size_of_chunk + 12 + TAG_LEN) as usize];
    input_file.seek(io::SeekFrom::Start(index as u64 * CHUNK_SIZE as u64))?;

    let range_of_data = 12..(size_of_chunk + 12);
    input_file.read_exact(&mut data[range_of_data.clone()])?;

    let (nonce, tag) = encrypt_v2_in_memory(&mut data[range_of_data], key_bytes, index, should_use_counter_nonce)?;

    // Append tag to the end of the data
    data[size_of_chunk as usize + 12..].copy_from_slice(tag.as_ref());

    // Append nonce to the beginning of the data
    data[0..12].copy_from_slice(&nonce);

    let hash = digest(&SHA512, &data);
    let hash = hex::encode(hash);

    // Write data to output file if it exists
    if let Some(output) = output {
        let mut output_file = std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(output)?;

        output_file.write_all(&data)?;

        return Ok((None, hash));
    } else {
        return Ok((Some(data), hash));
    }
}

/*
The lowest level function for encrypting data in memory.
This function encrypts the data in place and returns the nonce, key, and tag used.

The data will be extended to include the tag at the end.
*/
fn encrypt_v2_in_memory<'a>(
    data: &mut [u8],
    key_bytes: &[u8; 32],
    index: usize,
    should_use_counter_nonce: bool,
) -> Result<([u8; 12], Tag), CryptoError> {
    if data.len() > CHUNK_SIZE {
        return Err(CryptoError::Io(io::Error::new(
            io::ErrorKind::InvalidData,
            "Input data too large",
        )));
    }

    // Nonce generation
    let nonce_bytes = if should_use_counter_nonce {
        generate_counter_iv(index.try_into().unwrap())
    } else {
        let mut nonce = [0; 12];
        rand::SystemRandom::new().fill(&mut nonce)?;
        nonce
    };

    let sealing_key = aead::UnboundKey::new(&aead::AES_256_GCM, key_bytes)?;
    let nonce = aead::Nonce::try_assume_unique_for_key(&nonce_bytes)?;
    let sealing_key = aead::LessSafeKey::new(sealing_key);

    let tag = sealing_key.seal_in_place_separate_tag(nonce, aead::Aad::empty(), data)?;

    Ok((nonce_bytes, tag))
}

#[cfg(test)]
mod tests {
    use crate::crypto::{file_decrypt::decrypt_v2_in_memory, generate_rand_key};
    use memory_stats::memory_stats;

    use super::*;

    #[test]
    fn test_encrypt_v2_in_memory() {
        let input = "tests/out/test.txt";
        let output = "tests/out/test.out.enc";

        println!(
            "Current memory usage: {} MB",
            memory_stats().unwrap().physical_mem / 1024
        );
        let mut data = std::fs::read(input).unwrap();
        let key = generate_rand_key().unwrap();
        let (nonce, tag) = encrypt_v2_in_memory(&mut data, &key, 0, true).unwrap();
        println!(
            "Current memory usage: {} MB after encrypt",
            memory_stats().unwrap().physical_mem / 1024
        );

        let mut nonce_appended = [nonce.to_vec(), data, tag.as_ref().to_vec()].concat();
        let decrypted = decrypt_v2_in_memory(&mut nonce_appended, &key).unwrap();

        // Compare decrypted data with original data
        let original = std::fs::read(input).unwrap();
        assert_eq!(original, decrypted);
    }

    #[test]
    fn test_encrypt_file_output_to_file() {
        let input = "tests/out/test.txt";
        let output = "tests/out/test.out.enc";

        println!(
            "Current memory usage: {} MB",
            memory_stats().unwrap().physical_mem / 1024
        );
        let key = generate_rand_key().unwrap();
        let _ = encrypt_v2_from_file(input, Some(output), &key, 0, true).unwrap();
        println!(
            "Current memory usage: {} MB after encrypt",
            memory_stats().unwrap().physical_mem / 1024
        );

        let mut data = std::fs::read(output).unwrap();
        let decrypted = decrypt_v2_in_memory(&mut data, &key).unwrap();
        println!(
            "Current memory usage: {} MB after decrypt",
            memory_stats().unwrap().physical_mem / 1024
        );

        // Compare decrypted data with original data
        let original = std::fs::read(input).unwrap();
        assert_eq!(original, decrypted);

        // Remove output file
        std::fs::remove_file(output).unwrap();

        // Test memory usage
        // Create vector of 3 MB
        let mut data = vec![0; 1024 * 1024 * 3];
        // fill with random data
        rand::SystemRandom::new().fill(&mut data).unwrap();
        println!(
            "Current memory usage: {} MB after random allocations",
            memory_stats().unwrap().physical_mem / 1024
        );
    }

    #[test]
    fn test_encrypt_file_in_memory() {
        let input = "tests/out/test.txt";

        println!(
            "Current memory usage: {} MB",
            memory_stats().unwrap().physical_mem / 1024
        );
        let key = generate_rand_key().unwrap();
        let data = encrypt_v2_from_file(input, None, &key, 0, false).unwrap();
        let current_memory_after_encrypt = memory_stats().unwrap().physical_mem / 1024;
        println!(
            "Current memory usage: {} MB after encrypt",
            current_memory_after_encrypt
        );

        let mut data = data.0.unwrap();
        let decrypted = decrypt_v2_in_memory(&mut data, &key).unwrap();
        let memory_after_decrypt = memory_stats().unwrap().physical_mem / 1024;
        println!(
            "Current memory usage: {} MB after decrypt",
            memory_after_decrypt
        );

        // Compare decrypted data with original data
        let original = std::fs::read(input).unwrap();
        assert_eq!(original, decrypted);
    }
}

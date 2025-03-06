use std::error::Error;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;
use bytes::{Bytes, BytesMut};
use ring::aead::{self};
use tokio::io::{AsyncSeekExt, AsyncWriteExt};

pub fn stream_decrypt_data(
    input: &Path,
    output: &Path,
    key: &str,
    version: i32,
    index: Option<usize>,
    should_clear: bool,
) -> Result<(), Box<dyn Error>> {
    // Key string must be 32 bytes
    if key.len() != 32 {
        return Err(Box::new(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Invalid key length",
        )));
    }

    // Common validation
    let key_bytes = key.as_bytes();
    if !input.exists() {
        return Err(Box::new(io::Error::new(
            io::ErrorKind::NotFound,
            "Input file not found",
        )));
    }

    if should_clear && output.exists() {
        fs::remove_file(output)?;
    }

    match version {
        // 1 => decrypt_v1(input, output, key_bytes, index),
        1 => panic!("V1 is unsupported"),
        2 => decrypt_v2(input, output, key_bytes, index),
        _ => Err(Box::new(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Unsupported version",
        ))),
    }
}

fn decrypt_v2(
    input: &Path,
    output: &Path,
    key_bytes: &[u8],
    index: Option<usize>,
) -> Result<(), Box<dyn Error>> {
    let mut input_file = File::open(input)?;
    let metadata = input_file.metadata()?;
    let file_size = metadata.len();

    if file_size < 12 + 16 {
        return Err(Box::new(io::Error::new(
            io::ErrorKind::InvalidData,
            "Input file too small",
        )));
    }

    // Read whole file into memory
    let mut data: BytesMut = BytesMut::with_capacity(file_size as usize);
    data.resize(file_size as usize, 0);
    input_file.read_exact(&mut data)?;

    // Decrypt
    let fin_buf = decrypt_v2_bytes(&mut data, key_bytes)?;

    write_output(output, &fin_buf, index)
}

#[deprecated(
    since = "1.0.0",
    note = "Please use the Bytes version"
)]
pub fn decrypt_v2_in_memory<'a>(
    data: &'a mut [u8],
    key_bytes: &'a [u8],
) -> Result<&'a mut [u8], Box<dyn Error>> {
    if data.len() < 12 + 16 {
        return Err(Box::new(io::Error::new(
            io::ErrorKind::InvalidData,
            "Input data too small",
        )));
    }

    // Read IV (first 12 bytes)
    let iv = &data[0..12];

    // Decrypt
    let unbound_key = aead::UnboundKey::new(&aead::AES_256_GCM, &key_bytes)
        .map_err(|_| "Invalid key length")?;
    let nonce = aead::Nonce::assume_unique_for_key(iv.try_into().expect("slice with incorrect length"));
    let aad = aead::Aad::empty();

    let key = aead::LessSafeKey::new(unbound_key);
    let fin_buf = key.open_in_place(nonce, aad, &mut data[12..]).map_err(|_| "Decryption failed")?;

    Ok(fin_buf)
}

pub fn decrypt_v2_bytes(
    data: &mut BytesMut,
    key_bytes: &[u8],
) -> Result<BytesMut, Box<dyn Error>> {
    if data.len() < 12 + 16 {
        return Err(Box::new(io::Error::new(
            io::ErrorKind::InvalidData,
            "Input data too small",
        )));
    }

    // Read IV (first 12 bytes)
    let iv = &data[0..12];

    // Decrypt
    let unbound_key = aead::UnboundKey::new(&aead::AES_256_GCM, &key_bytes)
        .map_err(|_| "Invalid key length")?;
    let nonce = aead::Nonce::assume_unique_for_key(iv.try_into().expect("slice with incorrect length"));
    let aad = aead::Aad::empty();

    let key = aead::LessSafeKey::new(unbound_key);
    let mut bytes_mod = data.split_off(12);

    let fin_buf = key.open_in_place(nonce, aad, &mut bytes_mod).map_err(|_| "Decryption failed")?;

    // Remove IV from the data
    let fin_buf_len = fin_buf.len();
    bytes_mod.truncate(fin_buf_len);

    Ok(bytes_mod)
}

pub fn write_output(output: &Path, data: &[u8], index: Option<usize>) -> Result<(), Box<dyn Error>> {
    let mut output_file = OpenOptions::new()
        .create(true)
        .write(true)
        .open(output)?;

    if let Some(idx) = index {
        let offset = (idx * super::CHUNK_SIZE) as u64;
        output_file.seek(SeekFrom::Start(offset))?;
    }

    output_file.write_all(data)?;
    Ok(())
}

// Use tokio to create async writes
pub async fn write_output_async(output: &Path, data: &[u8], index: Option<usize>) -> Result<(), Box<dyn Error>> {
    let mut output_file = tokio::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(output).await?;

    if let Some(idx) = index {
        let offset = (idx * super::CHUNK_SIZE) as u64;
        output_file.seek(SeekFrom::Start(offset)).await?;
    }

    output_file.write_all(data).await?;
    Ok(())
}

#[cfg(test)]
mod tests{
    use super::*;
    use std::fs::remove_file;
    use memory_stats::memory_stats;

    #[test]
    fn test_stream_decrypt_data() {
        let input = Path::new("tests/out/test.enc");
        let output = Path::new("tests/out/test.dec.txt");
        let key = "abcd".repeat(8);
        let version = 2;
        let index = None;
        let should_clear = true;


        println!(
            "Current memory usage: {} MB",
            memory_stats().unwrap().physical_mem / 1024
        );
        let result = stream_decrypt_data(input, output, &key, version, index, should_clear);
        println!(
            "Current memory usage: {} MB",
            memory_stats().unwrap().physical_mem / 1024
        );
        
        println!("{:?}", result);
        assert!(result.is_ok());

        // Confirm that the output is 1024 * 1024 0x41
        let mut output_file = File::open(output).unwrap();
        let mut output_data = vec![0u8; 1024 * 1024];
        output_file.read_exact( &mut output_data).unwrap();

        assert_eq!(output_data, vec![0x41; 1024 * 1024]);
    }

    fn assert_send<T: Send>(_: T) {}

    #[test]
    fn test_send() {
        assert_send(decrypt_v2_bytes);
        assert_send(write_output);
        assert_send(write_output_async);
    }
}
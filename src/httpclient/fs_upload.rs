use std::sync::Arc;

use crate::{crypto::CHUNK_SIZE, error::FilenSDKError, filensdk::MAX_UPLOAD_THREADS, mod_private::upload::mark_upload_as_done};
use serde::{Deserialize, Serialize};

use super::{endpoints::string_url, httpclient::upload_from_memory, FsURL};

#[derive(Serialize, Deserialize)]
struct FileMetadata {
    name: String,
    size: Option<u64>,
    mime: Option<String>,
    #[serde(serialize_with = "serialize_bytes_as_string")]
    #[serde(deserialize_with = "deserialize_string_as_bytes")]
    key: [u8; 32],
    last_modified: Option<i64>,
    hash: Option<String>,
}

/*
My reasoning for not using a String is for a possibility of using any u8 array of 32 bytes rather
than being limited to the alphanumeric characters.
*/
pub fn serialize_bytes_as_string<S>(key: &[u8; 32], serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(String::from_utf8_lossy(key).as_ref())
}

pub fn deserialize_string_as_bytes<'de, D>(deserializer: D) -> Result<[u8; 32], D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: &str = Deserialize::deserialize(deserializer)?;
    let bytes = s.as_bytes();
    let mut key = [0; 32];
    key.copy_from_slice(&bytes);
    Ok(key)
}

pub async fn upload_file(
    input_file: &str,
    filen_parent: &str,
    name: &str,
    client: Arc<reqwest::Client>,
    api_key: &str,
    master_key: &str,
) -> Result<String, FilenSDKError> {
    // Does file exist?
    if !std::path::Path::new(&input_file).exists() {
        return Err(FilenSDKError::FileDoesNotExist { file: input_file.to_string() });
    }

    // Create tokio runtime using builder to configure multi-threaded runtime
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let _guard = rt.enter();

    let file = tokio::fs::File::open(input_file).await.unwrap();

    // File stats
    let metadata = file.metadata().await.unwrap();
    let file_size = metadata.len();
    let last_modified = metadata.modified().unwrap();
    let mime = mime_guess::from_path(input_file).first_or_octet_stream().to_string();

    // Generate shared key used for encryption
    let key = crate::crypto::generate_rand_key()?;

    let uuid = uuid::Uuid::new_v4().to_string();
    let file_name = name.to_string();

    // Calculate number of chunks there will be
    let chunks = (file_size as f64 / CHUNK_SIZE as f64).ceil() as usize;

    // Create metadata
    let metadata = FileMetadata {
        name: file_name.clone(),
        size: Some(file_size),
        mime: Some(mime.clone()),
        key: key.clone(),
        last_modified: Some(last_modified.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64),
        hash: None,
    };

    let key_str = String::from_utf8(key.to_vec()).unwrap();

    // Encrypt metadata
    let name_enc = crate::crypto::metadata::encrypt_metadata(&file_name.as_bytes(), &key_str)?;
    let mime_enc = crate::crypto::metadata::encrypt_metadata(&mime.as_bytes(), &key_str)?;
    let name_hashed = crate::crypto::metadata::hash_fn(&file_name.to_lowercase())?;
    let size_enc = crate::crypto::metadata::encrypt_metadata(&file_size.to_string().as_bytes(), &key_str)?;
    let metadata_json = serde_json::to_string(&metadata).map_err(|_| FilenSDKError::SerdeJsonError { err_msg: "Failed to serialize metadata".to_string(), err_str: "".to_string() })?;
    let metadata_enc = crate::crypto::metadata::encrypt_metadata(&metadata_json.as_bytes(), master_key)?;

    // Tokio channel for sending chunks
    let (tx, mut rx) = tokio::sync::mpsc::channel::<(usize, Vec<u8>)>(MAX_UPLOAD_THREADS);

    let input_file_clone = input_file.to_string();
    // Start encrypt thread
    tokio::spawn(async move {
        for i in 0..chunks {
            let data = crate::crypto::file_encrypt::encrypt_v2_from_file(&input_file_clone, None, &key, i).unwrap();
            tx.send((i, data.unwrap())).await.unwrap();
        }
    });

    let upload_key = String::from_utf8(crate::crypto::generate_rand_key().unwrap().to_vec()).unwrap();

    // Start upload threads
    let (tx_upload, mut rx_upload) = tokio::sync::mpsc::channel::<usize>(MAX_UPLOAD_THREADS);
    for _i in 0..chunks {
        let (index, data) = rx.recv().await.unwrap();
        let tx_upload = tx_upload.clone();
        let uuid = uuid.clone();
        // let key = key.clone();
        // let filen_parent = filen_parent.to_string();
        // let name_enc = name_enc.clone();
        // let mime_enc = mime_enc.clone();
        // let name_hashed = name_hashed.clone();
        // let size_enc = size_enc.clone();
        // let metadata_enc = metadata_enc.clone();
        let client = client.clone();
        let filen_parent = filen_parent.to_string();
        let api_key = api_key.to_string();
        let upload_key = upload_key.clone();

        tokio::spawn(async move {
            // Get sha512 hash of chunk
            let hash = ring::digest::digest(&ring::digest::SHA512, &data);
            let hash = hex::encode(hash.as_ref());

            let url = FsURL::Igest(uuid.clone(), upload_key, index as u64, filen_parent.clone(), hash.clone());
            let response = upload_from_memory(url, &client, data.into(), &api_key).await;

            match response {
                Ok(_) => {
                    tx_upload.send(index).await.unwrap();
                },
                Err(e) => {
                    println!("Error uploading chunk {}: {:?}", index, e);
                }
            }
        });
    }

    drop(tx_upload);

    while let Some(index) = rx_upload.recv().await {
        println!("Uploaded chunk {}", index);
    }

    // Mark upload as done
    mark_upload_as_done(
        uuid.clone(),
        String::from_utf8(name_enc).unwrap(),
        name_hashed,
        String::from_utf8(size_enc).unwrap(),
        chunks as i64,
        String::from_utf8(mime_enc).unwrap(),
        "false".to_string(),
        String::from_utf8(metadata_enc).unwrap(),
        upload_key,
        &client,
        &api_key,
    ).await?;

    Ok(uuid)
}

#[cfg(test)]
mod tests {
    use super::*;

    // #[test]
    // fn test_metadata_serialization() {
    //     let generated_key = crate::crypto::generate_rand_key().unwrap();
    //     let metadata = FileMetadata {
    //         name: "test.txt".to_string(),
    //         size: Some(100),
    //         mime: Some("text/plain".to_string()),
    //         key: generated_key,
    //         last_modified: Some(0),
    //         hash: None,
    //     };

    //     println!("{:?}", serde_json::to_string(&metadata).unwrap());
    // }

    #[async_std::test]
    async fn test_upload_file() {
        let input_file = "tests/out/test.txt";
        let filensdk = crate::filensdk::FilenSDK::new();
        
        dotenv::dotenv().ok();
        filensdk.import_credentials(dotenv::var("TEST_CRED_IMPORT").unwrap());

        let filen_parent = filensdk.base_folder().unwrap();
        // generate random file name
        let name = uuid::Uuid::new_v4().to_string() + ".txt";

        let result = upload_file(input_file, &filen_parent, &name, filensdk.client.clone(), &filensdk.api_key().unwrap(), &filensdk.master_key().unwrap()).await;
        assert!(result.is_ok());

        let uuid = result.unwrap();

        // Download file
        let download_path = "tests/out/test_download_out";
        let file_info = filensdk.file_info(uuid.clone()).await.unwrap();

        let decrypted_metadata = crate::crypto::metadata::decrypt_metadata(&file_info.metadata.as_bytes(), &filensdk.master_key().unwrap()).unwrap();
        let decrypted_metadata_str = String::from_utf8(decrypted_metadata).unwrap();
        let metadata: FileMetadata = serde_json::from_str(&decrypted_metadata_str).unwrap();

        let download_result = filensdk.download_file_low_disk(
            uuid.clone(),
            file_info.region,
            file_info.bucket,
            String::from_utf8(metadata.key.to_vec()).unwrap(),
            download_path.to_string(),
            metadata.name.clone(),
            metadata.size.unwrap(),
        ).await;

        // Compare files
        let file = std::fs::File::open(input_file).unwrap();
        let mut reader = std::io::BufReader::new(file);
        let mut buffer = Vec::new();
        std::io::Read::read_to_end(&mut reader, &mut buffer).unwrap();

        let file = std::fs::File::open(download_path.to_owned() + "/" + &metadata.name.to_owned()).unwrap();
        let mut reader = std::io::BufReader::new(file);
        let mut buffer_download = Vec::new();
        std::io::Read::read_to_end(&mut reader, &mut buffer_download).unwrap();

        assert_eq!(buffer, buffer_download);
    }
}
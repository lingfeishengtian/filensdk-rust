use std::{fs::File, hash, sync::Arc};

use crate::{
    crypto::CHUNK_SIZE, error::FilenSDKError, filensdk::MAX_UPLOAD_THREADS,
    responses::fs::UploadChunkResponse, FilenSDK,
};
use serde::{Deserialize, Serialize};

use super::{endpoints::string_url, httpclient::upload_from_memory, FsURL};

#[derive(Serialize, Deserialize)]
pub struct FileMetadata {
    pub name: String,
    pub size: Option<u64>,
    pub mime: Option<String>,
    #[serde(serialize_with = "serialize_bytes_as_string")]
    #[serde(deserialize_with = "deserialize_string_as_bytes")]
    pub key: [u8; 32],
    pub last_modified: Option<i64>,
    pub hash: Option<String>,
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

impl FilenSDK {
    pub async fn upload_file<T, B>(
        &self,
        input_file: &str,
        filen_parent: &str,
        name: &str,
        encrypt_data: impl (Fn(u64, &[u8; 32]) -> (T, String)) + Send + 'static,
        http_upload_data: impl Fn(FsURL, T) -> B + Send + Sync + Clone + 'static,
    ) -> Result<String, FilenSDKError>
    where
        T: Send + 'static,
        B: std::future::Future<Output = Result<UploadChunkResponse, FilenSDKError>>
            + Send
            + 'static,
    {
        // Does file exist?
        if !std::path::Path::new(&input_file).exists() {
            return Err(FilenSDKError::FileDoesNotExist {
                file: input_file.to_string(),
            });
        }

        // Create tokio runtime using builder to configure multi-threaded runtime
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        let _guard = rt.enter();

        let file = File::open(input_file).unwrap();

        // File stats
        let metadata = file.metadata().unwrap();
        let file_size = metadata.len();
        let last_modified = metadata.modified().unwrap();
        let mime = mime_guess::from_path(input_file)
            .first_or_octet_stream()
            .to_string();

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
            last_modified: Some(
                last_modified
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64,
            ),
            hash: None,
        };

        let key_str = String::from_utf8(key.to_vec()).unwrap();

        // Encrypt metadata
        let name_enc = crate::crypto::metadata::encrypt_metadata(&file_name.as_bytes(), &key_str)?;
        let mime_enc = crate::crypto::metadata::encrypt_metadata(&mime.as_bytes(), &key_str)?;
        let name_hashed = crate::crypto::metadata::hash_fn(&file_name.to_lowercase())?;
        let size_enc =
            crate::crypto::metadata::encrypt_metadata(&file_size.to_string().as_bytes(), &key_str)?;
        let metadata_json =
            serde_json::to_string(&metadata).map_err(|_| FilenSDKError::SerdeJsonError {
                err_msg: "Failed to serialize metadata".to_string(),
                err_str: "".to_string(),
            })?;
        let metadata_enc =
            crate::crypto::metadata::encrypt_metadata(&metadata_json.as_bytes(), &self.master_key()?)?;

        // Tokio channel for sending chunks
        let (tx, mut rx) = tokio::sync::mpsc::channel::<(usize, (T, String))>(MAX_UPLOAD_THREADS);

        // Start encrypt thread
        tokio::spawn(async move {
            for i in 0..chunks {
                // let data = crate::crypto::file_encrypt::encrypt_v2_from_file(&input_file_clone, None, &key, i).unwrap();
                let data = encrypt_data(i as u64, &key);
                tx.send((i, data)).await.unwrap();
            }
        });

        let upload_key =
            String::from_utf8(crate::crypto::generate_rand_key().unwrap().to_vec()).unwrap();

        // Start upload threads
        let (tx_upload, mut rx_upload) = tokio::sync::mpsc::channel::<usize>(MAX_UPLOAD_THREADS);
        for i in 0..chunks {
            let (index, data) = rt.block_on(rx.recv()).unwrap();
            if i > MAX_UPLOAD_THREADS {
                rt.block_on(rx_upload.recv());
            }

            let tx_upload = tx_upload.clone();
            let uuid = uuid.clone();
            let filen_parent = filen_parent.to_string();
            let upload_key = upload_key.clone();
            let http_upload_data = http_upload_data.clone();

            tokio::spawn(async move {
                // // Get sha512 hash of chunk
                // let hash = ring::digest::digest(&ring::digest::SHA512, &data);
                // let hash = hex::encode(hash.as_ref());

                let (data, hash) = data;
                let url = FsURL::Igest(
                    uuid.clone(),
                    upload_key,
                    index as u64,
                    filen_parent.clone(),
                    hash.clone(),
                );
                // let response = upload_from_memory(url, &client, data.into(), &api_key).await;
                let response = http_upload_data(url, data).await;

                match response {
                    Ok(_) => {
                        tx_upload.send(index).await.unwrap();
                    }
                    Err(e) => {
                        println!("Error uploading chunk {}: {:?}", index, e);
                    }
                }
            });
        }

        drop(tx_upload);

        while let Some(index) = rt.block_on(rx_upload.recv()) {
            println!("Uploaded chunk {}", index);
        }

        // Mark upload as done
        self.mark_upload_as_done(
            uuid.clone(),
            String::from_utf8(name_enc).unwrap(),
            name_hashed,
            String::from_utf8(size_enc).unwrap(),
            chunks as i64,
            String::from_utf8(mime_enc).unwrap(),
            "false".to_string(),
            String::from_utf8(metadata_enc).unwrap(),
            upload_key
        )?;

        Ok(uuid)
    }
}
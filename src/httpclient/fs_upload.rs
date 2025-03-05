use std::{fs::File, hash, sync::Arc};

use crate::{
    crypto::CHUNK_SIZE, error::FilenSDKError, filensdk::MAX_UPLOAD_THREADS, requests::fs::FileMetadata, responses::fs::UploadChunkResponse, FilenSDK
};

use super::FsURL;

impl FilenSDK {
    pub async fn upload_file_generic<T, B>(
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
            key: key.to_vec(),
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
        let (tx_upload, mut rx_upload) = tokio::sync::mpsc::channel::<(usize, bool)>(MAX_UPLOAD_THREADS);
        for i in 0..chunks {
            let (index, data) = rx.recv().await.unwrap();
            if i > MAX_UPLOAD_THREADS {
                let resp = rx_upload.recv().await.unwrap();
                crate::return_function_on_result_fail!(resp);
            }

            let tx_upload = tx_upload.clone();
            let uuid = uuid.clone();
            let filen_parent = filen_parent.to_string();
            let upload_key = upload_key.clone();
            let http_upload_data = http_upload_data.clone();

            let upload_semaphore = self.upload_semaphore.clone();

            tokio::spawn(async move {
                // Obtain upload permit
                let _permit = upload_semaphore.acquire().await.unwrap();

                let (data, hash) = data;
                let url = FsURL::Igest(
                    uuid.clone(),
                    upload_key,
                    index as u64,
                    filen_parent.clone(),
                    hash.clone(),
                );
                let response = http_upload_data(url, data).await;

                match response {
                    Ok(_) => {
                        tx_upload.send((index, true)).await.unwrap();
                    }
                    Err(e) => {
                        tx_upload.send((index, false)).await.unwrap();
                    }
                }
            });
        }

        drop(tx_upload);

        while let Some(resp) = rx_upload.recv().await {
            crate::return_function_on_result_fail!(resp);
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
        ).await?;

        Ok(uuid)
    }
}
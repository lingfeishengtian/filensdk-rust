// Create one thread for encrypting

// main thread waits for finished encrypted data

// create another thread for uploading which is limited by the upload semaphore and MAX UPLOAD THREADS

use std::io::Read;

use bytes::Bytes;

use crate::{httpclient::httpclient::{upload_from_file, upload_from_memory}, FilenSDK};

#[uniffi::export]
impl FilenSDK {
    pub async fn upload_file_low_disk(
        &self,
        input_file: String,
        filen_parent: String,
        name: String,
        should_use_counter_nonce: bool,
    ) -> Result<String, crate::error::FilenSDKError> {
        let client = self.client.clone();
        let api_key = self.api_key()?;
        let input_file = input_file.to_string();

        self.upload_file(
            &input_file.clone(),
            &filen_parent,
            &name,
            move |i, key| {
                let input_file = input_file.clone();
                // let nonce = if should_use_counter_nonce {
                //     crate::crypto::generate_counter_iv(i)
                // } else {
                //     crate::crypto::generate_rand_iv().unwrap()
                // };
                let (encrypted_data, hash) = crate::crypto::file_encrypt::encrypt_v2_from_file(
                    &input_file,
                    None,
                    key,
                    i as usize,
                )
                .unwrap();
                (encrypted_data, hash)
            },
            move |url, data| {
                let client = client.clone();
                {
                    let value = api_key.clone();
                    async move { upload_from_memory(url, &client, data.unwrap(), &value).await }
                }
            },
        )
        .await
    }


    pub async fn upload_file_low_memory(
        &self,
        input_file: String,
        filen_parent: String,
        name: String,
        tmp_output_dir: String,
        should_use_counter_nonce: bool,
    ) -> Result<String, crate::error::FilenSDKError> {
        let client = self.client.clone();
        let api_key = self.api_key()?;
        let input_file = input_file.to_string();

        // Create tmp directory if it doesn't exist
        if !std::path::Path::new(&tmp_output_dir).exists() {
            std::fs::create_dir(&tmp_output_dir).unwrap();
        }

        self.upload_file(
            &input_file.clone(),
            &filen_parent,
            &name,
            move |i, key| {
                let input_file = input_file.clone();
                // let nonce = if should_use_counter_nonce {
                //     crate::crypto::generate_counter_iv(i)
                // } else {
                //     crate::crypto::generate_rand_iv().unwrap()
                // };
                let output_file = tmp_output_dir.to_string() + "/" + &i.to_string();
                let (_encrypted_data, hash) = crate::crypto::file_encrypt::encrypt_v2_from_file(
                    &input_file,
                    Some(&output_file),
                    key,
                    i as usize,
                )
                .unwrap();
                (output_file, hash)
            },
            move |url, data| {
                let client = client.clone();
                {
                    let value = api_key.clone();
                    async move { 
                        let fut = upload_from_file(url, &client, &data, &value).await;
                        std::fs::remove_file(&data).unwrap();
                        fut
                    }
                }
            },
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use crate::httpclient::fs_upload::FileMetadata;

    use super::*;

    #[async_std::test]
    async fn test_upload_file() {
        let input_file = "tests/out/test.txt";
        let filensdk = crate::filensdk::FilenSDK::new();

        dotenv::dotenv().ok();
        filensdk.import_credentials(dotenv::var("TEST_CRED_IMPORT").unwrap());

        let filen_parent = filensdk.base_folder().unwrap();
        // generate random file name
        let name = uuid::Uuid::new_v4().to_string() + ".txt";

        let result = filensdk
            // .upload_file_low_disk(input_file.to_string(), filen_parent, name, true)
            .upload_file_low_memory(input_file.to_string(), filen_parent, name, "tests/tmp/test_up".to_string(), true)
            .await;
        assert!(result.is_ok());

        let uuid = result.unwrap();

        // Download file
        let download_path = "tests/out/test_download_out";
        let file_info = filensdk.file_info(uuid.clone()).await.unwrap();

        let decrypted_metadata = crate::crypto::metadata::decrypt_metadata(
            &file_info.metadata.as_bytes(),
            &filensdk.master_key().unwrap(),
        )
        .unwrap();
        let decrypted_metadata_str = String::from_utf8(decrypted_metadata).unwrap();
        let metadata: FileMetadata = serde_json::from_str(&decrypted_metadata_str).unwrap();

        let download_result = filensdk
            .download_file_low_disk(
                uuid.clone(),
                file_info.region,
                file_info.bucket,
                String::from_utf8(metadata.key.to_vec()).unwrap(),
                download_path.to_string(),
                metadata.name.clone(),
                metadata.size.unwrap(),
            )
            .await;

        // Compare files
        let file = std::fs::File::open(input_file).unwrap();
        let mut reader = std::io::BufReader::new(file);
        let mut buffer = Vec::new();
        std::io::Read::read_to_end(&mut reader, &mut buffer).unwrap();

        let file = std::fs::File::open(download_path.to_owned() + "/" + &metadata.name.to_owned())
            .unwrap();
        let mut reader = std::io::BufReader::new(file);
        let mut buffer_download = Vec::new();
        std::io::Read::read_to_end(&mut reader, &mut buffer_download).unwrap();

        assert_eq!(buffer, buffer_download);
    }
}

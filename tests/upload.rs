
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_upload_file() {
        // let input_file = "tests/out/test.txt";
        let input_file = "tests/out/Pixelmon-1.16.5-9.1.12-ARM-Mac-FIxed.jar";
        let filensdk = filensdk::FilenSDK::new();

        dotenv::dotenv().ok();
        filensdk.import_credentials(dotenv::var("TEST_CRED_IMPORT").unwrap());

        let filen_parent = filensdk.base_folder().unwrap();
        // generate random file name
        let name = uuid::Uuid::new_v4().to_string() + ".txt";

        let result = filensdk
            // .upload_file_low_disk(input_file.to_string(), filen_parent, name, true)
            .upload_file_low_memory_blocking(input_file.to_string(), filen_parent, name, "tests/tmp/test_up".to_string(), true);
        assert!(result.is_ok());

        let uuid = result.unwrap();

        // Download file
        let download_path = "tests/out/test_download_out";
        let _download_result = filensdk
            .download_file_blocking(uuid, download_path.to_string());

        // Compare files
        let file = std::fs::File::open(input_file).unwrap();
        let mut reader = std::io::BufReader::new(file);
        let mut buffer = Vec::new();
        std::io::Read::read_to_end(&mut reader, &mut buffer).unwrap();

        let file = std::fs::File::open(download_path.to_owned())
            .unwrap();
        let mut reader = std::io::BufReader::new(file);
        let mut buffer_download = Vec::new();
        std::io::Read::read_to_end(&mut reader, &mut buffer_download).unwrap();

        assert_eq!(buffer, buffer_download);
    }
}

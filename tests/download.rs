#[cfg(test)]
mod tests {
    use filensdk::download::FileByteRange;
    use filensdk::download_stream::FilenDownloadStream;
    use filensdk::FilenSDK;
    use filensdk::CHUNK_SIZE;
    use test_context::test_context;
    use test_context::AsyncTestContext;

    use super::*;
    use std::fs::remove_file;
    use std::fs::File;
    use std::io::Seek;
    use std::io::Write;
    use std::sync::Arc;

    struct DownloadTestContext {
        sdk: Arc<FilenSDK>,
        uuid: String,
        region: String,
        bucket: String,
        key: String,
        output_dir: String,
        file_name: String,
        file_size: u64,
        current_time: std::time::SystemTime,
        byte_range: Option<FileByteRange>,
    }

    impl AsyncTestContext for DownloadTestContext {
        async fn setup() -> Self {
            dotenv::dotenv().ok();
            let sdk = Arc::new(FilenSDK::new());

            // Import credentials from dotenv
            let creds = std::env::var("TEST_CRED_IMPORT").unwrap();
            sdk.import_credentials(creds);

            let uuid = std::env::var("TEST_UUID").unwrap();
            let region = std::env::var("TEST_REGION").unwrap();
            let bucket = std::env::var("TEST_BUCKET").unwrap();
            let key = std::env::var("TEST_KEY").unwrap();
            let output_dir = std::env::var("TEST_OUTPUT_DIR").unwrap();
            let file_name = std::env::var("TEST_FILE_NAME").unwrap();
            let file_size: u64 = std::env::var("TEST_FILE_SIZE").unwrap().parse().unwrap();

            let current_time = std::time::SystemTime::now();

            DownloadTestContext {
                sdk,
                uuid,
                region,
                bucket,
                key,
                output_dir,
                file_name,
                file_size,
                current_time,
                byte_range: None,
            }
        }

        async fn teardown(self) {
            let elapsed = self.current_time.elapsed().unwrap();

            println!(
                "Download speed: {} MB/s",
                self.file_size as f64 / elapsed.as_secs_f64() / 1024.0 / 1024.0
            );

            // Compare sha256 of downloaded file with original using ring::digest
            let sha = std::env::var("TEST_FILE_SHA256").unwrap();

            let file_path = format!("{}/{}", self.output_dir, self.file_name);
            let digest = sha256_digest(&file_path);

            assert_eq!(digest, sha);
        }
    }

    fn sha256_digest(file_path: &str) -> String {
        let file = File::open(file_path).unwrap();
        let mut reader = std::io::BufReader::new(file);
        let mut context = ring::digest::Context::new(&ring::digest::SHA256);
        let mut buffer = [0; 1024];
        loop {
            let count = std::io::Read::read(&mut reader, &mut buffer).unwrap();
            if count == 0 {
                break;
            }
            context.update(&buffer[..count]);
        }
        let digest = context.finish();
        let digest = digest.as_ref();
        hex::encode(digest)
    }

    fn sha256_digest_partial(file_path: &str, index: u64) -> String {
        let mut file = File::open(file_path).unwrap();
        file.seek(std::io::SeekFrom::Start(index * CHUNK_SIZE as u64))
            .unwrap();
        let mut reader = std::io::BufReader::new(file);
        let mut context = ring::digest::Context::new(&ring::digest::SHA256);
        let mut buffer = vec![0; CHUNK_SIZE];

        let count = std::io::Read::read(&mut reader, &mut buffer).unwrap();
        context.update(&buffer[..count]);

        let digest = context.finish();
        let digest = digest.as_ref();
        hex::encode(digest)
    }

    #[test_context(DownloadTestContext, skip_teardown)]
    #[async_std::test]
    async fn test_failed_path(ctx: DownloadTestContext) {
        dotenv::dotenv().ok();

        // sdk.download_file_low_disk(uuid, region, bucket, key, output_dir.clone(), file_name.clone(), file_size).await;
        // sdk.download_file_low_memory(uuid, region, bucket, key, output_dir.clone(), "tests/tmp".to_string(), file_name.clone(), file_size).await;
        let res = ctx
            .sdk
            .download_file_blocking(ctx.uuid.clone(), ctx.output_dir);

        assert!(res.is_err());
    }

    #[test_context(DownloadTestContext)]
    #[async_std::test]
    async fn test_download_file(ctx: &mut DownloadTestContext) {
        dotenv::dotenv().ok();

        // sdk.download_file_low_disk(uuid, region, bucket, key, output_dir.clone(), file_name.clone(), file_size).await;
        // sdk.download_file_low_memory(uuid, region, bucket, key, output_dir.clone(), "tests/tmp".to_string(), file_name.clone(), file_size).await;
        let file_path = format!("{}/{}", ctx.output_dir, ctx.file_name);

        ctx.sdk
            .download_file_blocking(ctx.uuid.clone(), file_path)
            .unwrap();
    }

    #[test_context(DownloadTestContext)]
    #[test]
    fn test_download_file_low_memory(ctx: &mut DownloadTestContext) {
        dotenv::dotenv().ok();
        let file_path = format!("{}/{}", ctx.output_dir, ctx.file_name);
        // Remove file if it exists
        remove_file(&file_path).unwrap_or_default();

        ctx.sdk
            .download_file_low_memory_blocking(ctx.uuid.clone(), file_path, ctx.output_dir.clone() + "/tmp")
            .unwrap();

        // Confirm tmp dir was made
        assert!(std::path::Path::new(&(ctx.output_dir.clone() + "/tmp")).exists());
        std::fs::remove_dir_all(ctx.output_dir.clone() + "/tmp").unwrap();
    }

    #[test_context(DownloadTestContext, skip_teardown)]
    #[async_std::test]
    async fn test_chunked_download(ctx: DownloadTestContext) {
        let random_start = rand::random::<u64>() % (ctx.file_size / 2);
        let random_end = rand::random::<u64>() % (ctx.file_size / 2) + ctx.file_size / 2;

        let byte_range = ctx
            .sdk
            .download_file_chunked_blocking(
                ctx.uuid,
                format!("{}/{}-chunked", ctx.output_dir, ctx.file_name),
                Some(random_start),
                Some(random_end),
            )
            .unwrap();

        // Check that output_dir/file_name exists and has the correct sha256
        let file_path = format!("{}/{}", ctx.output_dir, ctx.file_name);
        let sha = std::env::var("TEST_FILE_SHA256").unwrap();
        let digest = sha256_digest(&file_path);
        assert_eq!(digest, sha);

        // Compare partial sha256 with original
        let start_ind = byte_range.start_byte / CHUNK_SIZE as u64;
        let end_ind = byte_range.end_byte / CHUNK_SIZE as u64;

        for i in start_ind..end_ind {
            let partial_digest = sha256_digest_partial(&file_path, i);
            let original_digest = sha256_digest_partial(&file_path, i);
            assert_eq!(partial_digest, original_digest);
        }
    }

    #[test_context(DownloadTestContext)]
    #[test]
    fn test_streamed_download(ctx: &mut DownloadTestContext) {
        // Delete old file since we append to it
        let file_path = format!("{}/{}", ctx.output_dir, ctx.file_name);
        remove_file(&file_path).unwrap_or_default();

        let stream = FilenDownloadStream::new(
            ctx.file_size,
            0,
            ctx.sdk.clone(),
            &ctx.region,
            &ctx.bucket,
            &ctx.uuid,
            ctx.key.clone(),
        );

        while let Ok(chunk) = stream.next_blocking() {
            let file_path = format!("{}/{}", ctx.output_dir, ctx.file_name);
            let mut file = std::fs::OpenOptions::new()
                .append(true)
                .create(true)
                .open(&file_path)
                .unwrap();
            file.write_all(&chunk).unwrap();
        }
    }

    #[test]
    fn test_pathing() {
        let path = std::path::Path::new("tests/tmp");
        // Get parent directory
        let parent = path.parent().unwrap();
        assert_eq!(parent.to_str().unwrap(), "tests");

        let path = std::path::Path::new("/tmp");
        // Get parent directory
        let parent = path.parent().unwrap();
        assert_eq!(parent.to_str().unwrap(), "/");
    }
}

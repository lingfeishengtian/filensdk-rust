#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use filensdk::{httpserver::{config::FilenHttpServerConfig, FilenHttpService}, responses::fs::{DirContentFolder, DirContentUpload}, FilenSDK};
    use futures::{StreamExt, TryStreamExt};

    #[async_std::test]
    async fn test_retrieve_auth_info() {
        let sdk = FilenSDK::new();
        dotenv::dotenv().ok();
        let email = std::env::var("TEST_EMAIL").expect("TEST_EMAIL must be set");

        let status = sdk.retrieve_auth_info(&email).await;
        assert_eq!(status.is_ok(), true);
        assert_eq!(status.unwrap().email, email);
    }

    #[async_std::test]
    async fn test_login() {
        if std::env::var("SHOULD_TEST_LOGIN").unwrap_or_else(|_| "false".to_string()) == "false" {
            return;
        }
        let sdk = FilenSDK::new();
        let email = std::env::var("TEST_EMAIL").expect("TEST_EMAIL must be set");
        let password = std::env::var("TEST_PASSWORD").expect("TEST_PASSWORD must be set");
        let otp = std::env::var("TEST_OTP").ok();
        let status = sdk.login(&email, &password, otp.map(|s| s.to_string())).await;

        println!("{:?}", sdk.export_credentials());

        assert_eq!(status.is_ok(), true);
        // Confirm UserID from dotenv
        let user_id = std::env::var("TEST_USER_ID").expect("TEST_USER_ID must be set");
        assert_eq!(sdk.user_id(), user_id);

    }

    #[test]
    fn test_cred_import() {
        let sdk = FilenSDK::new();
        dotenv::dotenv().ok();
        let credentials = std::env::var("TEST_CRED_IMPORT").expect("TEST_CRED_IMPORT must be set");
        sdk.import_credentials(credentials);
        
        // Confirm UserID from dotenv
        let user_id = std::env::var("TEST_USER_ID").expect("TEST_USER_ID must be set");
        assert_eq!(sdk.user_id(), user_id);
    }

    #[test]
    fn test_http_server() {
        let sdk = Arc::new(FilenSDK::new());
        dotenv::dotenv().ok();
        let credentials = std::env::var("TEST_CRED_IMPORT").expect("TEST_CRED_IMPORT must be set");
        sdk.import_credentials(credentials);
        
        FilenHttpService::new(sdk, FilenHttpServerConfig {
            port: 8080,
            max_connections: 100,
            timeout: 5,
        }, "tests/tmp".to_string()).start_server();
    }

    use memory_stats::memory_stats;
    use tokio_util::io::StreamReader;
}
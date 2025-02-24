#[cfg(test)]
mod tests {
    use filensdk::FilenSDK;

    #[async_std::test]
    async fn test_retrieve_auth_info() {
        let sdk = FilenSDK::new();
        dotenv::dotenv().ok();
        let email = std::env::var("TEST_EMAIL").expect("TEST_EMAIL must be set");
    }

    #[async_std::test]
    async fn test_login() {
        dotenv::dotenv().ok();
        if std::env::var("SHOULD_TEST_LOGIN").unwrap_or_else(|_| "false".to_string()) == "false" {
            return;
        }
        let sdk = FilenSDK::new();
        let email = std::env::var("TEST_EMAIL").expect("TEST_EMAIL must be set");
        let password = std::env::var("TEST_PASSWORD").expect("TEST_PASSWORD must be set");
        let otp = std::env::var("TEST_OTP").ok();
        let status = sdk.login(&email, &password, otp.map(|s| s.to_string())).await;

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
}

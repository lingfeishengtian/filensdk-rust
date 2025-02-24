#[cfg(test)]
mod tests {
    use filensdk::FilenSDK;

    #[async_std::test]
    async fn test_retrieve_auth_info() {
        let sdk = FilenSDK::new();
        println!("{:?}", sdk.retrieve_auth_info("tester@gmail.com").await);
    }
}

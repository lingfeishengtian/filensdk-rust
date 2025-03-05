#[derive(uniffi::Record, Clone)]
pub struct FilenHttpServerConfig {
    pub port: u16,
    pub max_connections: u64,
    pub timeout: u64,
}
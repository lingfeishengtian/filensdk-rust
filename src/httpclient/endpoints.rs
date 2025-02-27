use url::Url;

use super::httpclient::RequestMethod;

// TODO: Move these to constants
const BASE_GATEWAY_URL: &str = "https://gateway.filen.io";
const EGEST_URLS: [&str; 8] = [
    "https://egest.filen.io",
    "https://egest.filen.net",
    "https://egest.filen-1.net",
    "https://egest.filen-2.net",
    "https://egest.filen-3.net",
    "https://egest.filen-4.net",
    "https://egest.filen-5.net",
    "https://egest.filen-6.net",
];

const INGEST_URLS: [&str; 8] = [
    "https://ingest.filen.io",
    "https://ingest.filen.net",
    "https://ingest.filen-1.net",
    "https://ingest.filen-2.net",
    "https://ingest.filen-3.net",
    "https://ingest.filen-4.net",
    "https://ingest.filen-5.net",
    "https://ingest.filen-6.net",
];

pub struct FilenEndpoint {
    pub endpoint: &'static str,
    pub method: RequestMethod,
}

impl FilenEndpoint {
    pub fn convert_full_url(&self) -> String {
        Url::parse(&format!(
            "{}/{}",
            BASE_GATEWAY_URL,
            self.endpoint.trim_start_matches("/")
        ))
        .unwrap()
        .to_string()
    }
}

macro_rules! define_endpoints {
    ($($name:ident => ($endpoint:expr, $method:ident)),* $(,)?) => {
        #[derive(Debug)]
        pub enum Endpoints {
            $(
                $name,
            )*
        }

        impl Endpoints {
            pub fn get_endpoint(&self) -> FilenEndpoint {
                match self {
                    $(
                        Endpoints::$name => FilenEndpoint {
                            endpoint: $endpoint,
                            method: RequestMethod::$method,
                        },
                    )*
                }
            }
        }
    };
}

define_endpoints![
    // Auth
    AuthInfo => ("/v3/auth/info", POST),
    Login => ("/v3/login", POST),
    UserInfo => ("/v3/user/info", GET),
    
    // Files
    UploadDone => ("/v3/upload/done", POST),
    // File
    FileInfo => ("/v3/file", POST),
];

#[derive(Debug)]
pub enum FsURL {
    Egest(String, String, String, u64),
    Igest(String, String, u64, String, String), 
}

pub fn string_url(url: &FsURL) -> Url {
    match url {
        FsURL::Egest(region, bucket, uuid, index) => {
            let egest_url = EGEST_URLS[*index as usize % EGEST_URLS.len()];
            Url::parse(&format!(
                "{}/{}/{}/{}/{}",
                egest_url, region, bucket, uuid, index
            ))
            .unwrap()
        },
        FsURL::Igest(uuid, upload_key, index, parent, hash) => {
            let ingest_url = INGEST_URLS[*index as usize % INGEST_URLS.len()];
            Url::parse(&format!(
                "{}/v3/upload?uuid={}&index={}&uploadKey={}&parent={}&hash={}",
                ingest_url, uuid.to_lowercase(), index, upload_key, parent.to_lowercase(), hash
            ))
            .unwrap()
        }
    }
}

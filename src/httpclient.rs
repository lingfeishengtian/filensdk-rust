use std::collections::HashMap;
use url::Url;

const BASE_GATEWAY_URL: &str = "https://gateway.filen.io";

#[derive(serde::Deserialize)]
pub struct FilenResponse<T> {
    pub status: bool,
    pub message: String,
    pub code: Option<String>,
    pub data: Option<T>,
}

pub enum FilenURL {
    baseUrl(String),
    // TODO: Ingest and Egest
}

pub enum RequestMethod {
    GET,
    POST,
    PUT,
    DELETE,
}

macro_rules! generate_request_methods {
    ($($method:ident; $req_method:ident),*) => {
        $(
            pub fn $method<T>(
                url: FilenURL,
                parameters: Option<HashMap<&str, &str>>,
                api_key: Option<&str>,
                body: Option<HashMap<&str, &str>>,
            ) -> Result<FilenResponse<T>, reqwest::Error>
            where
                T: serde::de::DeserializeOwned,
            {
                make_request(url, parameters, api_key, body, RequestMethod::$req_method)
            }
        )*
    };
}

generate_request_methods!(
    make_get_request; GET, 
    make_post_request; POST, 
    make_put_request; PUT, 
    make_delete_request; DELETE
);

pub fn make_request<T>(
    url: FilenURL,
    parameters: Option<HashMap<&str, &str>>,
    api_key: Option<&str>,
    body: Option<HashMap<&str, &str>>,
    method: RequestMethod,
) -> Result<FilenResponse<T>, reqwest::Error>
where
    T: serde::de::DeserializeOwned,
{
    let client = reqwest::Client::new();
    let mut request = match method {
        RequestMethod::GET => client.get(string_url(url)),
        RequestMethod::POST => client.post(string_url(url)),
        RequestMethod::PUT => client.put(string_url(url)),
        RequestMethod::DELETE => client.delete(string_url(url)),
    };

    if let Some(parameters) = parameters {
        request = request.query(&parameters);
    }

    if let Some(api_key) = api_key
    {
        request = request.header("Authorization", "Bearer ".to_owned() + api_key);
    }

    if let Some(body) = body {
        request = request.form(&body);
    }

    let rt = tokio::runtime::Runtime::new().unwrap();
    let response = rt.block_on(request.send());
    let response_text = rt.block_on(response?.text());

    if let Ok(response_text) = response_text {
        let response_json: FilenResponse<T> = serde_json::from_str(&response_text).unwrap();
        return Ok(response_json);
    } else {
        return Err(response_text.unwrap_err());
    }
}

fn string_url(url: FilenURL) -> Url {
    match url {
        FilenURL::baseUrl(endpoint) => Url::parse(&format!("{}/{}", BASE_GATEWAY_URL, endpoint.trim_start_matches("/"))).unwrap(),
    }
}
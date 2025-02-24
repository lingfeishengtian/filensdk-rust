use std::collections::HashMap;
use url::Url;

use crate::error::FilenSDKError;

const BASE_GATEWAY_URL: &str = "https://gateway.filen.io";

#[derive(serde::Deserialize, Debug)]
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
            pub fn $method<T, U>(
                url: FilenURL,
                parameters: Option<HashMap<&str, &str>>,
                api_key: Option<&str>,
                body: Option<U>,
            ) -> Result<T, FilenSDKError> where
            T: serde::de::DeserializeOwned + std::fmt::Debug,
            U: serde::Serialize,
            {
                make_request(url, parameters, api_key, body, RequestMethod::$req_method)
            }
        )*
    };
}

#[derive(serde::Serialize)]
pub struct HttpClientNone { }

pub fn http_none() -> Option<HttpClientNone> {
    None
}

generate_request_methods!(
    make_get_request; GET, 
    make_post_request; POST, 
    make_put_request; PUT, 
    make_delete_request; DELETE
);

pub fn make_request<T, U>(
    url: FilenURL,
    parameters: Option<HashMap<&str, &str>>,
    api_key: Option<&str>,
    body: Option<U>,
    method: RequestMethod,
) -> Result<T, FilenSDKError> where
    T: serde::de::DeserializeOwned + std::fmt::Debug,
    U: serde::Serialize,
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
        request = request.bearer_auth(api_key);
    }

    if let Some(body) = body {
        request = request.json(&body);
    }

    let rt = tokio::runtime::Runtime::new().unwrap();
    let response = rt.block_on(request.send());
    let response_text = match response {
        Ok(response) => response.text(),
        Err(e) => return Err(FilenSDKError::ReqwestError { err_str: e.to_string() }),
    };

    let response_text = rt.block_on(response_text);

    if let Ok(response_text) = response_text {
        let response_json = serde_json::from_str(&response_text);
        if response_json.is_err() {
            return Err(FilenSDKError::SerdeJsonError { err_str: response_text, err_msg: response_json.unwrap_err().to_string() });
        }

        let response_json: FilenResponse<T> = response_json.unwrap();
        if response_json.status && response_json.data.is_some() {
            return Ok(response_json.data.unwrap());
        } else {
            return Err(FilenSDKError::APIError { message: response_json.message, code: response_json.code });
        }
    } else {
        return Err(FilenSDKError::ReqwestError { err_str: response_text.unwrap_err().to_string() });
    }
}

fn string_url(url: FilenURL) -> Url {
    match url {
        FilenURL::baseUrl(endpoint) => Url::parse(&format!("{}/{}", BASE_GATEWAY_URL, endpoint.trim_start_matches("/"))).unwrap(),
    }
}
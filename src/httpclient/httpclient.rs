use std::collections::HashMap;
use bytes::Bytes;
use reqwest::Client;
use tokio::io::AsyncWriteExt;
use url::Url;

use crate::error::FilenSDKError;

use super::{endpoints::{string_url, Endpoints}, FsURL};


#[derive(serde::Deserialize, Debug)]
pub struct FilenResponse<T> {
    pub status: bool,
    pub message: String,
    pub code: Option<String>,
    pub data: Option<T>,
}

pub enum RequestMethod {
    GET,
    POST,
    PUT,
    DELETE,
}

#[derive(serde::Serialize)]
pub struct HttpClientNone { }

pub fn http_none() -> Option<HttpClientNone> {
    None
}

/*
This function assumes that a tokio runtime is already running.
*/
pub async fn download_into_memory(url: FsURL, client: &reqwest::Client) -> Result<Bytes, FilenSDKError> {
    let request = client.get(string_url(&url));

    let response = request.send().await;
    let response_text = match response {
        Ok(response) => response.bytes().await,
        Err(e) => return Err(FilenSDKError::ReqwestError { err_str: e.to_string() }),
    };
    
    let response_text = response_text.unwrap();
    Ok(response_text)
}

pub async fn download_to_file_streamed(url: FsURL, client: &reqwest::Client, file_path: &str) -> Result<String, FilenSDKError> {
    let request = client.get(string_url(&url));

    let response = request.send().await;
    let mut response = match response {
        Ok(response) => response,
        Err(e) => return Err(FilenSDKError::ReqwestError { err_str: e.to_string() }),
    };

    let mut file = tokio::fs::File::create(file_path).await.unwrap();
    while let Some(chunk) = response.chunk().await.unwrap() {
        file.write_all(&chunk).await.unwrap();
    }

    Ok(file_path.to_string())
}

pub fn make_request<T, U>(
    url: Endpoints,
    client: Option<&reqwest::Client>,
    parameters: Option<HashMap<&str, &str>>,
    api_key: Option<&str>,
    body: Option<U>,
) -> Result<T, FilenSDKError> where
    T: serde::de::DeserializeOwned + std::fmt::Debug,
    U: serde::Serialize,
{
    let client: &reqwest::Client = match client {
        Some(client) => client,
        None => &Client::new(),
    };

    let endpoint = url.get_endpoint();
    let url = endpoint.convert_full_url();

    let mut request = match endpoint.method {
        RequestMethod::GET => client.get(url),
        RequestMethod::POST => client.post(url),
        RequestMethod::PUT => client.put(url),
        RequestMethod::DELETE => client.delete(url),
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
use bytes::Bytes;
use reqwest::Client;
use std::collections::HashMap;
use tokio::io::AsyncWriteExt;
use url::Url;

use crate::{error::FilenSDKError, responses::fs::UploadChunkResponse};

use super::{
    endpoints::{string_url, Endpoints},
    FsURL,
};

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
pub struct HttpClientNone {}

pub fn http_none() -> Option<HttpClientNone> {
    None
}

/*
This function assumes that a tokio runtime is already running.
*/
pub async fn download_into_memory(
    url: &FsURL,
    client: &reqwest::Client,
) -> Result<Bytes, FilenSDKError> {
    let request = client.get(string_url(&url));

    let response = request.send().await;
    let response_text = match response {
        Ok(response) => response.bytes().await,
        Err(e) => {
            return Err(FilenSDKError::ReqwestError {
                err_str: e.to_string(),
            })
        }
    };

    Ok(response_text.map_err(|e| FilenSDKError::ReqwestError {
        err_str: e.to_string(),
    })?)
}

pub async fn download_to_file_streamed(
    url: &FsURL,
    client: &reqwest::Client,
    file_path: &str,
) -> Result<String, FilenSDKError> {
    let request = client.get(string_url(&url));

    let response = request.send().await;
    let mut response = match response {
        Ok(response) => response,
        Err(e) => {
            return Err(FilenSDKError::ReqwestError {
                err_str: e.to_string(),
            })
        }
    };

    let mut file = tokio::fs::File::create(file_path).await?;
    while let Some(chunk) = response.chunk().await? {
        file.write_all(&chunk).await?;
    }

    Ok(file_path.to_string())
}

pub async fn upload_from_memory(
    url: FsURL,
    client: &reqwest::Client,
    data: Bytes,
    api_key: &str,
) -> Result<UploadChunkResponse, FilenSDKError> {
    let request = client.post(string_url(&url)).body(data);

    // Setup API key
    let request = request.bearer_auth(api_key);
    let request = request.header("Accept", "application/json");

    let response = request.send().await;
    let response = match response {
        Ok(response) => response,
        Err(e) => {
            return Err(FilenSDKError::ReqwestError {
                err_str: e.to_string(),
            })
        }
    };

    handle_upload_response(response).await
}

pub async fn upload_from_file(
    url: FsURL,
    client: &reqwest::Client,
    file_path: &str,
    api_key: &str,
) -> Result<UploadChunkResponse, FilenSDKError> {
    let file = tokio::fs::File::open(file_path).await.unwrap();

    // Use streaming
    let request = client
        .post(string_url(&url))
        .body(reqwest::Body::wrap_stream(
            tokio_util::io::ReaderStream::new(file),
        ));

    // Setup API key
    let request = request.bearer_auth(api_key);
    let request = request.header("Accept", "application/json");

    let response = request.send().await;
    let response = match response {
        Ok(response) => response,
        Err(e) => {
            return Err(FilenSDKError::ReqwestError {
                err_str: e.to_string(),
            })
        }
    };

    handle_upload_response(response).await
}

async fn handle_upload_response(
    response: reqwest::Response,
) -> Result<UploadChunkResponse, FilenSDKError> {
    let response_text = serde_json::from_str(&response.text().await.unwrap());
    if response_text.is_err() {
        return Err(FilenSDKError::SerdeJsonError {
            err_str: "Failed to parse response text".to_string(),
            err_msg: response_text.unwrap_err().to_string(),
        });
    }

    let response_text: FilenResponse<UploadChunkResponse> = response_text.unwrap();
    if response_text.status && response_text.data.is_some() {
        return Ok(response_text.data.unwrap());
    } else {
        return Err(FilenSDKError::SerdeJsonError {
            err_str: "Failed to parse response text".to_string(),
            err_msg: response_text.message,
        });
    }
}

pub async fn make_request<T, U>(
    url: Endpoints,
    client: Option<&reqwest::Client>,
    parameters: Option<HashMap<&str, &str>>,
    api_key: Option<&str>,
    body: Option<U>,
) -> Result<T, FilenSDKError>
where
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

    if let Some(api_key) = api_key {
        request = request.bearer_auth(api_key);
    }

    if let Some(body) = body {
        request = request.json(&body);
    }

    let response = request.send().await;
    let response_text = match response {
        Ok(response) => response.text(),
        Err(e) => {
            return Err(FilenSDKError::ReqwestError {
                err_str: e.to_string(),
            })
        }
    };

    let response_text = response_text.await;

    if let Ok(response_text) = response_text {
        let response_json = serde_json::from_str(&response_text);
        if response_json.is_err() {
            return Err(FilenSDKError::SerdeJsonError {
                err_str: response_text,
                err_msg: response_json.unwrap_err().to_string(),
            });
        }

        let response_json: FilenResponse<T> = response_json.unwrap();
        if response_json.status && response_json.data.is_some() {
            return Ok(response_json.data.unwrap());
        } else {
            return Err(FilenSDKError::APIError {
                message: response_json.message,
                code: response_json.code,
            });
        }
    } else {
        return Err(FilenSDKError::ReqwestError {
            err_str: response_text.unwrap_err().to_string(),
        });
    }
}

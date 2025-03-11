use std::boxed;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use async_stream::stream;
use bytes::Bytes;
use futures_core::Stream;
use http_body_util::combinators::BoxBody;
use http_body_util::{Full, StreamBody};
use hyper::server::conn::http1;
use hyper::server::conn::http2;
use hyper::service::{service_fn, Service};
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::{TokioExecutor, TokioIo, TokioTimer};
use tokio::io::{AsyncReadExt, AsyncSeekExt};

use crate::error::FilenSDKError;
use crate::{filensdk, FilenSDK, CHUNK_SIZE};

use super::config::{self, FilenHttpServerConfig};

#[derive(uniffi::Object)]
pub struct FilenHttpService {
    filen_sdk: Arc<FilenSDK>,
    configuration: FilenHttpServerConfig,
    tmp_dir: String,
}

// TODO: Organize this

/*
Public methods
*/
#[uniffi::export]
impl FilenHttpService {
    #[uniffi::constructor]
    pub fn new(
        filen_sdk: Arc<FilenSDK>,
        configuration: FilenHttpServerConfig,
        tmp_dir: String,
    ) -> Self {
        Self {
            filen_sdk,
            configuration,
            tmp_dir,
        }
    }

    // Blocking function to start the server
    pub fn start_server(&self) {
        // Begin Tokio runtime multi-threaded server
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let addr = SocketAddr::from(([127, 0, 0, 1], self.configuration.port));
            let tcp_listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

            loop {
                let (socket, _) = tcp_listener.accept().await.unwrap();
                let filen_sdk = self.filen_sdk.clone();
                let cloned_tmp_dir = self.tmp_dir.clone();

                let io = TokioIo::new(socket);
                let config = self.configuration.clone();
                tokio::spawn(async move {
                    if let Err(err) = http1::Builder::new().serve_connection(io, service_fn(move |req| {
                        let filen_sdk = filen_sdk.clone();
                        let cloned_tmp_dir = cloned_tmp_dir.clone();
                        async move {
                            hello(req, filen_sdk, &cloned_tmp_dir).await
                        }
                    })).await {
                        eprintln!("Error serving connection: {}", err);
                    }
                });
            }
        });
    }
}

fn convert_byte_stream_to_hyper_stream(
    stream: impl Stream<Item = Result<Bytes, FilenSDKError>>,
) -> impl Stream<Item = Result<hyper::body::Frame<Bytes>, Infallible>> {
    use futures::stream::StreamExt;

    stream.map(|item| match item {
        Ok(bytes) => Ok(hyper::body::Frame::data(bytes)),
        Err(_) => Ok(hyper::body::Frame::data(Bytes::new())),
    })
}

async fn hello(
    req: Request<hyper::body::Incoming>,
    filen_sdk: Arc<FilenSDK>,
    tmpdir: &str,
) -> Result<Response<BoxBody<Bytes, Infallible>>, Infallible> {
    // Get query parameter "uuid"
    let uuid: Option<String> = match req.uri().query() {
        Some(query) => {
            let query_pairs = url::form_urlencoded::parse(query.as_bytes());
            let mut uuid = None;

            for (key, value) in query_pairs {
                if key == "uuid" {
                    uuid = Some(value.to_string());
                    break;
                }
            }

            uuid
        }
        None => None,
    };

    if uuid.is_none() {
        println!("Missing query parameter 'uuid'");

        panic!("Missing query parameter 'uuid'");
        // return Ok(Response::builder()
        //         .status(StatusCode::BAD_REQUEST)
        //         .header("Accept-Ranges", "bytes")
        //         .body(Full::from(Bytes::from("Missing query parameter 'uuid'")))
        //         .unwrap());
    }

    let uuid = uuid.unwrap();

    println!("Downloading file with UUID: {}", uuid);

    let file_info = filen_sdk.file_info(uuid.clone()).await.unwrap();
    let size = file_info.size;

    // Check for range header
    let range = match req.headers().get("Range") {
        Some(range) => {
            println!("Range header: {:?}", range);
            let range = range.to_str().unwrap();
            let range = range.replace("bytes=", "");
            let range: Vec<&str> = range.split("-").collect();

            let start = range[0].parse::<u64>().unwrap_or_else(|_| 0);
            let end = range[1].parse::<u64>().unwrap_or_else(|_| size);

            Some((start, end))
        }
        None => {
            println!("No range header");
            None
        }
    };

    let (start_byte, end_byte) = match range {
        Some((start, end)) => (Some(start), Some(end)),
        None => (None, None),
    };

    // Create dir
    tokio::fs::create_dir_all(tmpdir).await.unwrap();

    let tmpdir_clone = tmpdir.to_owned();

    let stream = filen_sdk.read_ahead_download_stream(
        size,
        start_byte.unwrap_or(0),
        file_info.region,
        file_info.bucket,
        uuid,
        String::from_utf8(file_info.key).unwrap(),
    );

    let body_stream = StreamBody::new(convert_byte_stream_to_hyper_stream(stream));
    let boxed = BoxBody::new(body_stream);

    Ok(Response::builder()
        .status(StatusCode::PARTIAL_CONTENT)
        .header("Content-Type", "application/octet-stream")
        .header(
            "Content-Length",
            format!("{}", end_byte.unwrap_or(size) - start_byte.unwrap_or(0)),
        )
        .header("Accept-Ranges", "bytes")
        .header(
            "Content-Range",
            format!(
                "bytes {}-{}/{}",
                start_byte.unwrap_or(0),
                end_byte.unwrap_or(size),
                size
            ),
        )
        .body(boxed)
        .unwrap())
}
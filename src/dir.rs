use std::sync::{Arc, Mutex};

use streamed_json::iter_json_array;
use uniffi::FfiConverter;
use uniffi_shared_tokio_runtime_proc::uniffi_async_export;

use crate::{
    error::FilenSDKError,
    file::FilenFileDetailed,
    httpclient::{httpclient::construct_request, Endpoints},
    requests::fs::DirContentBody,
    responses::fs::StreamedDirContentResponse,
    FilenSDK,
};

/// This struct is necessary purely for an iterator wrapper for foreign code. However,
/// rust code can still use the iterator directly as an iterator.
#[derive(uniffi::Object)]
pub struct DirContentsIterator {
    internal_iter: Arc<
        Mutex<
            Box<
                dyn Iterator<Item = Result<StreamedDirContentResponse, std::io::Error>>
                    + Send
                    + Sync,
            >,
        >,
    >,
    master_key: String,
}

impl DirContentsIterator {
    pub fn new(
        iter: Box<
            dyn Iterator<Item = Result<StreamedDirContentResponse, std::io::Error>> + Send + Sync,
        >,
        master_key: String,
    ) -> Self {
        Self {
            internal_iter: Arc::new(Mutex::new(iter)),
            master_key,
        }
    }

    fn decrypt_get_response(
        &self,
        response: StreamedDirContentResponse,
    ) -> Result<DecryptedStreamedDirContentResponse, FilenSDKError> {
        match response {
            StreamedDirContentResponse::Uploads(upload) => {
                let decrypted_metadata =
                    FilenSDK::decrypt_metadata(upload.metadata, self.master_key.clone())?;
                let name = decrypted_metadata.name.clone();
                let path_for_name = std::path::Path::new(&name);
                Ok(DecryptedStreamedDirContentResponse::Uploads(
                    FilenFileDetailed {
                        uuid: upload.uuid,
                        region: upload.region,
                        bucket: upload.bucket,
                        name: decrypted_metadata.name,
                        size: upload.size,
                        mime: decrypted_metadata.mime.unwrap_or(
                            mime_guess::from_path(path_for_name)
                                .first()
                                .map(|m| m.to_string())
                                .unwrap_or("".to_owned()),
                        ),
                        key: decrypted_metadata.key,
                        last_modified: decrypted_metadata.last_modified,
                        parent: upload.parent,
                        versioned: None,
                        trash: false,
                        version: upload.version,
                    },
                ))
            }
            StreamedDirContentResponse::Folders(folder) => Ok(
                DecryptedStreamedDirContentResponse::Folders(FilenFolderDetailed {
                    uuid: folder.uuid,
                    name: String::from_utf8(crate::crypto::metadata::decrypt_metadata(
                        &folder.name.as_bytes(),
                        &self.master_key,
                    )?)?,
                    parent: folder.parent,
                    color: folder.color,
                    timestamp: folder.timestamp,
                    favorited: folder.favorited,
                    is_sync: folder.is_sync,
                    is_default: folder.is_default,
                }),
            ),
        }
    }
}

impl Iterator for DirContentsIterator {
    type Item = Result<DecryptedStreamedDirContentResponse, FilenSDKError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.internal_iter.lock().unwrap().next().map(|res| {
            res.map_err(|e| e.into())
                .and_then(|response| self.decrypt_get_response(response))
        })
    }
}

#[uniffi::export]
impl DirContentsIterator {
    pub fn next(&self) -> Result<Option<StreamedDirContentResponse>, FilenSDKError> {
        self.internal_iter
            .lock()
            .unwrap()
            .next()
            .map(|res| res.map_err(|e| e.into()))
            .transpose()
    }
}

#[derive(uniffi::Record)]
pub struct FilenFolderDetailed {
    uuid: String,
    name: String,
    parent: String,
    color: Option<String>,
    timestamp: u64,
    favorited: u64,
    is_sync: Option<u64>,
    is_default: Option<u64>,
}

#[derive(uniffi::Enum)]
pub enum DecryptedStreamedDirContentResponse {
    Uploads(FilenFileDetailed),
    Folders(FilenFolderDetailed),
}

#[uniffi_async_export]
impl FilenSDK {
    pub async fn dir_contents_iter(
        &self,
        uuid: String,
        folders_only: bool,
    ) -> Result<DirContentsIterator, FilenSDKError> {
        let constructed_request = construct_request(
            Endpoints::DirContent,
            Some(&self.client),
            None,
            Some(&self.api_key()?),
            Some(DirContentBody { uuid, folders_only }),
        )?;

        let response = constructed_request.send().await?;
        let byte_stream = response.bytes_stream();
        let reader =
            tokio_util::io::StreamReader::new(futures::TryStreamExt::map_err(byte_stream, |e| {
                std::io::Error::new(std::io::ErrorKind::Other, e)
            }));
        let sync_io_read = tokio_util::io::SyncIoBridge::new(reader);
        let json_iter = iter_json_array(sync_io_read);

        Ok(DirContentsIterator::new(Box::new(json_iter), self.master_key()?))
    }
}

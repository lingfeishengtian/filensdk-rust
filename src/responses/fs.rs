use serde::Deserialize;

use crate::response_struct;

use super::auth::AuthVersion;

response_struct! {
    MarkUploadAsDone {
        chunks: i64,
        size: i64,
    }

    UploadChunkResponse {
        bucket: String,
        region: String,
    }

    FileGetResponse {
        uuid: String,
        region: String,
        bucket: String,
        name_encrypted: String,
        name_hashed: String,
        size_encrypted: String,
        mime_encrypted: String,
        metadata: String,
        size: i64,
        parent: String,
        versioned: bool,
        trash: bool,
        version: AuthVersion,
    }

    DirContentUpload {
        uuid: String,
        metadata: String,
        rm: String,
        timestamp: u64,
        chunks: u64,
        size: u64,
        bucket: String,
        region: String,
        parent: String,
        version: AuthVersion,
        favorited: u64,
    }

    DirContentFolder {
        uuid: String,
        name: String,
        parent: String,
        color: Option<String>,
        timestamp: u64,
        favorited: u64,
        is_sync: Option<u64>,
        is_default: Option<u64>,
    }

    DirContentResponse {
        uploads: Vec<DirContentUpload>,
        folders: Vec<DirContentFolder>,
    }
}

#[derive(uniffi::Enum)]
#[streamed_json::streamed_json(lowerCamelCase)]
pub enum StreamedDirContentResponse {
    Uploads(DirContentUpload),
    Folders(DirContentFolder),
}

impl StreamedDirContentResponse {
    pub fn is_dir(&self) -> bool {
        match self {
            StreamedDirContentResponse::Uploads(_) => false,
            StreamedDirContentResponse::Folders(_) => true,
        }
    }
}
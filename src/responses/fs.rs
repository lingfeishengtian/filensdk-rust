use serde::Deserialize;

use crate::response_struct;

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
        version: i64,
    }
}
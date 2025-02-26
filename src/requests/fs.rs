use serde::Serialize;

use crate::{request_struct, responses::auth::AuthVersion};

request_struct! {
    MarkUploadAsDoneBody {
        uuid: String,
        name: String,
        name_hashed: String,
        size: String,
        chunks: i64,
        mime: String,
        rm: String,
        metadata: String,
        version: AuthVersion,
        upload_key: String,
    }
}
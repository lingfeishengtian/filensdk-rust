use serde::Deserialize;

use crate::response_struct;

response_struct! {
    MarkUploadAsDone {
        chunks: i64,
        size: i64,
    }
}
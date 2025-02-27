use crate::{
    error::FilenSDKError,
    httpclient::{make_request, Endpoints},
    requests::fs::MarkUploadAsDoneBody,
    responses::{auth::AuthVersion, fs::MarkUploadAsDone},
    FilenSDK,
};

pub async fn mark_upload_as_done(
    uuid: String,
    name: String,
    name_hashed: String,
    size: String,
    chunks: i64,
    mime: String,
    rm: String,
    metadata: String,
    upload_key: String,
    client: &reqwest::Client,
    api_key: &str,
) -> Result<MarkUploadAsDone, FilenSDKError> {
    make_request(
        Endpoints::UploadDone,
        Some(client),
        None,
        Some(api_key),
        Some(MarkUploadAsDoneBody {
            uuid,
            name,
            name_hashed,
            size,
            chunks,
            mime,
            rm,
            metadata,
            version: AuthVersion::V2,
            upload_key,
        }),
    )
}

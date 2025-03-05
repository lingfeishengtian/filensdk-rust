use crate::{
    error::FilenSDKError,
    httpclient::{make_request, Endpoints},
    requests::fs::MarkUploadAsDoneBody,
    responses::{auth::AuthVersion, fs::MarkUploadAsDone},
    FilenSDK,
};

impl FilenSDK {
    pub async fn mark_upload_as_done(
        &self,
        uuid: String,
        name: String,
        name_hashed: String,
        size: String,
        chunks: i64,
        mime: String,
        rm: String,
        metadata: String,
        upload_key: String,
    ) -> Result<MarkUploadAsDone, FilenSDKError> {
        make_request(
            Endpoints::UploadDone,
            Some(&self.client),
            None,
            Some(&self.api_key()?),
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
        ).await
    }
}

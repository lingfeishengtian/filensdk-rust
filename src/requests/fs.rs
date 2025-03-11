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

    FileInfoBody {
        uuid: String,
    }

    DirContentBody {
        uuid: String,
        folders_only: bool,
    }
}


#[derive(Serialize, serde::Deserialize, uniffi::Record)]
pub struct FileMetadata {
    pub name: String,
    pub size: Option<u64>,
    pub mime: Option<String>,
    #[serde(serialize_with = "serialize_bytes_as_string")]
    #[serde(deserialize_with = "deserialize_string_as_bytes")]
    pub key: Vec<u8>,
    pub last_modified: Option<i64>,
    pub hash: Option<String>,
}

pub fn serialize_bytes_as_string<S>(key: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    // Ensure key is 32 bytes
    if key.len() > 32 {
        return Err(serde::ser::Error::custom("Key is longer than 32 bytes"));
    }
    
    let mut key = key.clone();
    key.resize(32, 0);

    serializer.serialize_str(String::from_utf8_lossy(&key).as_ref())
}

pub fn deserialize_string_as_bytes<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = serde::Deserialize::deserialize(deserializer)?;
    let bytes = s.as_bytes();
    let mut key = vec![0; 32];
    key[..bytes.len()].copy_from_slice(&bytes);
    Ok(key)
}
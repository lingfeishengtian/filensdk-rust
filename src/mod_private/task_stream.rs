use std::{
    cmp::min, collections::VecDeque, convert::Infallible, ops::RangeInclusive, sync::Arc,
    time::Duration,
};

use bytes::Bytes;
use futures_core::Stream;
use range_set::RangeSet;
use tokio::task::JoinHandle;

use crate::{
    crypto::file_decrypt::{decrypt_v2_bytes, decrypt_v2_in_memory},
    error::FilenSDKError,
    httpclient::{download_into_memory, FsURL},
    mod_private::download::{DownloadFunctions, LowDiskDownloadFunctions},
    FilenSDK, CHUNK_SIZE,
};

use super::DOWNLOAD_RETRIES;

const MAX_READ_AHEAD_THREADS: u64 = 50;

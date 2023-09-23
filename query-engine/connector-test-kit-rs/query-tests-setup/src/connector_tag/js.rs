mod external_process;

use super::*;
use external_process::*;
use serde::de::DeserializeOwned;
use std::{collections::HashMap, sync::atomic::AtomicU64};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

pub(crate) async fn executor_process_request<T: DeserializeOwned>(
    method: &str,
    params: serde_json::Value,
) -> Result<T, Box<dyn std::error::Error + Send + Sync>> {
    EXTERNAL_PROCESS.request(method, params).await
}

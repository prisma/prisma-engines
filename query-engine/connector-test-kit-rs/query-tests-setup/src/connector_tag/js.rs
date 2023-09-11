mod node_process;

use super::*;
use node_process::*;
use serde::de::DeserializeOwned;
use std::{collections::HashMap, sync::atomic::AtomicU64};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

pub(crate) async fn executor_process_request<T: DeserializeOwned>(
    method: &str,
    params: serde_json::Value,
) -> Result<T, Box<dyn std::error::Error + Send + Sync>> {
    NODE_PROCESS.request(method, params).await
}

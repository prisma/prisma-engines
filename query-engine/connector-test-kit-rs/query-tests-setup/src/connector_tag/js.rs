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
    match EXTERNAL_PROCESS.request::<T>(method, params).await {
        Response::None => panic!("Missing result value"),
        Response::Ok(value) => Ok(value),
        Response::Err(error) => Err(error),
    }
}

pub(crate) async fn executor_process_request_no_return(
    method: &str,
    params: serde_json::Value,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match EXTERNAL_PROCESS.request::<()>(method, params).await {
        Response::None => Ok(()),
        Response::Ok(_) => panic!("There should be no result value"),
        Response::Err(error) => Err(error),
    }
}

use futures::compat::*;
use jsonrpc_core::IoHandler;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt};

pub async fn run(handler: &IoHandler) -> std::io::Result<()> {
    run_with_io(handler, tokio::io::stdin(), tokio::io::stdout()).await
}

async fn run_with_io(
    handler: &IoHandler,
    input: impl AsyncRead + Unpin,
    output: impl AsyncWrite + Unpin,
) -> std::io::Result<()> {
    let input = tokio::io::BufReader::new(input);
    let mut input_lines = input.lines();
    let mut output = tokio::io::BufWriter::new(output);

    while let Some(line) = input_lines.next_line().await? {
        let response = handle_request(&handler, &line).await;
        output.write_all(response.as_bytes()).await?;
        output.write_all(b"\n").await?;
        output.flush().await?;
    }

    Ok(())
}

/// Process a request asynchronously
async fn handle_request(io: &IoHandler, input: &str) -> String {
    let response = io.handle_request(input).compat().await;

    response
        .expect("jsonrpc-core returned an empty error")
        .unwrap_or_else(|| {
            tracing::info!("JSON RPC request produced no response: {:?}", input);
            String::from("")
        })
}

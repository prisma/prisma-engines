use jsonrpc_core::IoHandler;
use futures::compat::*;
use tokio::io::{AsyncWriteExt, AsyncBufReadExt};

pub struct ServerBuilder {
    handler: IoHandler,
}

impl ServerBuilder {
    /// Returns a new server instance
    pub fn new<T>(handler: T) -> Self
    where
        T: Into<IoHandler>,
    {
        ServerBuilder {
            handler: handler.into(),
        }
    }

    /// Run the server until EOF.
    pub async fn run(self) -> std::io::Result<()> {
        let stdin = tokio::io::BufReader::new(tokio::io::stdin());
        let mut stdin_lines = stdin.lines();
        let mut stdout = tokio::io::BufWriter::new(tokio::io::stdout());

        while let Some(line) = stdin_lines.next_line().await? {
            let response = handle_request(&self.handler, &line).await;
            stdout.write_all(response.as_bytes()).await?;
            stdout.write_all(b"\n").await?;
            stdout.flush().await?;
        }

        Ok(())
    }
}

/// Process a request asynchronously
async fn handle_request(io: &IoHandler, input: &str) -> String {
    let response = io.handle_request(input).compat().await;

    
    match response.expect("jsonrpc-core returned an empty error") {
        Some(res) => res,
        None =>  {
            tracing::info!("JSON RPC request produced no response: {:?}", input);
            String::from("")
        }
    }
}

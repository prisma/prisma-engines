//! This is a JSON-RPC server based on jsonrpc-core and async-std.

use async_std::{
    io::{BufRead, Write},
    prelude::*,
};
use futures03::compat::*;
use jsonrpc_core::IoHandler;

pub struct ServerBuilder {
    handler: jsonrpc_core::IoHandler,
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

    /// Will block until EOF is read or until an error occurs.
    /// The server reads from stdin line-by-line, one request is taken
    /// per line and each response is written to stdout on a new line.
    pub fn start_stdio(&self) {
        self.start(
            async_std::io::BufReader::new(async_std::io::stdin()),
            async_std::io::stdout(),
        )
    }

    /// Will block until EOF is read or until an error occurs.
    /// The server reads from `input` line-by-line, one request is taken
    /// per line and each response is written to `output` on a new line.
    pub fn start(&self, input: impl BufRead + Unpin, output: impl Write + Unpin) {
        async_std::task::block_on(self.serve(input, output)).unwrap();
    }

    async fn serve(
        &self,
        mut input: impl BufRead + Unpin,
        mut output: impl Write + Unpin,
    ) -> Result<(), failure::Error> {
        let mut buf = String::with_capacity(1024);

        while let Ok(len) = input.read_line(&mut buf).await {
            if len == 0 {
                // we have reached EOF
                break;
            }

            let response = self
                .handler
                .handle_request(&buf)
                .compat()
                .await
                .map_err(|_| failure::format_err!("Error during JSON-RPC request handling."))?
                .unwrap_or_else(String::new);
            output.write(response.as_bytes()).await?;
            output.write("\n".as_bytes()).await?;

            buf.clear();
        }

        Ok(())
    }
}

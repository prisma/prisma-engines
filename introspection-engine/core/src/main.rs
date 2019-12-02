mod connector_loader;
mod error;
mod rpc;

#[cfg(test)]
mod tests;

use rpc::{Rpc, RpcImpl};

use jsonrpc_core::*;
use jsonrpc_stdio_server::ServerBuilder;

fn main() {
    let mut io_handler = IoHandler::new();
    io_handler.extend_with(RpcImpl::new().to_delegate());

    let server = ServerBuilder::new(io_handler);
    server.build();
}

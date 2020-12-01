mod command_error;
mod error;
mod error_rendering;
mod rpc;

use jsonrpc_core::*;
use rpc::{Rpc, RpcImpl};

#[tokio::main]
async fn main() {
    use std::env;

    let arguments: Vec<String> = env::args().collect();

    if arguments.len() == 2 && arguments.iter().any(|i| i == "--version") {
        println!("introspection-core {}", env!("GIT_HASH"));
    } else {
        init_logger();
        user_facing_errors::set_panic_hook();

        let mut io_handler = IoHandler::new();
        io_handler.extend_with(RpcImpl::new().to_delegate());

        json_rpc_stdio::run(&io_handler).await.unwrap();
    };
}

fn init_logger() {
    use tracing_subscriber::{EnvFilter, FmtSubscriber};

    FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init()
}

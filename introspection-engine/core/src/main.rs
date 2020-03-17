mod command_error;
mod error;
mod error_rendering;
mod rpc;

use jsonrpc_core::*;
use rpc::{Rpc, RpcImpl};
use structopt::StructOpt;

#[derive(Debug, StructOpt, Clone)]
#[structopt(version = env!("GIT_HASH"))]
pub struct IntrospectionOpt {}

#[tokio::main]
async fn main() {
    init_logger();

    let _ = IntrospectionOpt::from_args();
    user_facing_errors::set_panic_hook();

    let mut io_handler = IoHandler::new();
    io_handler.extend_with(RpcImpl::new().to_delegate());

    json_rpc_stdio::run(&io_handler).await.unwrap();
}

fn init_logger() {
    use tracing_subscriber::{EnvFilter, FmtSubscriber};

    FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init()
}

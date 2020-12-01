mod command_error;
mod error;
mod error_rendering;
mod rpc;

use jsonrpc_core::*;
use rpc::{Rpc, RpcImpl};

struct IntrospectionArgs {
    pub version: bool,
}

#[tokio::main]
async fn main() {
    let mut args = pico_args::Arguments::from_env();
    let args = IntrospectionArgs {
        version: args.contains(["-v", "--version"]),
    };

    if args.version {
        println!("introspection-core {}", env!("GIT_HASH"));
    } else {
        init_logger();
        user_facing_errors::set_panic_hook();

        let mut io_handler = IoHandler::new();
        io_handler.extend_with(RpcImpl::new().to_delegate());

        json_rpc_stdio::run(&io_handler).await.unwrap();
    }
}

fn init_logger() {
    use tracing_subscriber::{EnvFilter, FmtSubscriber};

    FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init()
}

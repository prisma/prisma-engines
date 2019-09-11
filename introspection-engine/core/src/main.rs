mod connector_loader;
mod error;
mod rpc;

use error::*;
use introspection_connector::IntrospectionConnector;
use sql_introspection_connector::SqlIntrospectionConnector;
use std::io;

#[macro_use]
extern crate serde_derive;

fn main() {
    //    let mut input = String::new();
    //    io::stdin()
    //        .read_line(&mut input)
    //        .expect("Reading datasource url from stdin failed");
    //
    //    let data_source_url = input.trim_end_matches('\n'); // read_line appends a line break
    //
    //    doit(&data_source_url).expect("Introspection Failed");
    rpc::RpcApi::start()
}

fn doit(url: &str) -> CoreResult<()> {
    // FIXME: parse URL correctly via a to be built lib and pass database param;
    let data_model = connector_loader::load_connector(&url)?.introspect("")?;
    Ok(datamodel::render_to(&mut std::io::stdout().lock(), &data_model).unwrap())
}

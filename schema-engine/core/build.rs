use std::{env, path::Path};

fn main() {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let out_dir = Path::new(&out_dir);
    json_rpc_api_build::generate_rust_modules(out_dir).unwrap();
}

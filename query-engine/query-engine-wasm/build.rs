extern crate napi_build;

use std::{env, fs, path::PathBuf, process::Command};

fn store_git_commit_hash() {
    let out_path = PathBuf::from(env::var("OUT_DIR").expect("$OUT_DIR env var is not set"));

    let git_hash_cmd_output = Command::new("git").args(["rev-parse", "HEAD"]).output().unwrap();
    let git_hash = String::from_utf8(git_hash_cmd_output.stdout).unwrap();

    let git_hash_rs = format!("pub const GIT_HASH: &str = \"{}\";", git_hash);

    fs::write(out_path.join("generated_git_hash.rs"), &git_hash_rs)
        .expect("Couldn't write generated_git_hash.rs file!");

    println!("cargo:rerun-if-changed=build.rs");
}

fn main() {
    store_git_commit_hash();
}

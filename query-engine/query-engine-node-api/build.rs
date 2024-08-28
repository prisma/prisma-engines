extern crate napi_build;

use std::process::Command;

fn store_git_commit_hash() {
    let output = Command::new("git").args(["rev-parse", "HEAD"]).output().unwrap();

    // Sanity check on the output.
    if !output.status.success() {
        panic!(
            "Failed to get git commit hash.\nstderr: \n{}\nstdout {}\n",
            String::from_utf8(output.stderr).unwrap_or_default(),
            String::from_utf8(output.stdout).unwrap_or_default(),
        );
    }

    let git_hash = String::from_utf8(output.stdout).unwrap();
    println!("cargo:rustc-env=GIT_HASH={git_hash}");
}

fn main() {
    store_git_commit_hash();
    napi_build::setup()
}

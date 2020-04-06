use rustc_version::version;
use std::process::Command;

fn check_rust_version() {
    let rust_version = version().expect("Could not get rustc version");
    let expected_major_version = 1;
    let expected_minor_version = 40;

    assert_eq!(rust_version.major, expected_major_version);

    if rust_version.minor < expected_minor_version {
        panic!(
            "You don't have the right Rust version installed. This build expects at least version {}.{}.x",
            expected_major_version, expected_minor_version,
        )
    }
}

fn store_git_commit_hash() {
    let output = Command::new("git").args(&["rev-parse", "HEAD"]).output().unwrap();
    let git_hash = String::from_utf8(output.stdout).unwrap();
    println!("cargo:rustc-env=GIT_HASH={}", git_hash);
}

fn main() {
    check_rust_version();
    store_git_commit_hash();
}

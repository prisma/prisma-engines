use std::process::Command;

/// Store the current git commit hash in the `GIT_HASH` variable in rustc env.
/// If the `GIT_HASH` environment variable is already set, this function does nothing.
pub fn store_git_commit_hash_in_env() {
    if std::env::var("GIT_HASH").is_ok() {
        return;
    }

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

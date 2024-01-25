use query_engine_tests::TestResult;
use std::{env, path, process, sync::Mutex};

/// Runs the command in the background and performs the future given, then kills the process
pub(crate) async fn with_child_process<F>(command: &mut process::Command, f: F) -> TestResult<()>
where
    F: std::future::Future<Output = ()>,
{
    struct Cleaner<'a> {
        p: &'a mut std::process::Child,
    }
    impl<'a> Drop for Cleaner<'a> {
        fn drop(&mut self) {
            self.p.kill().expect("Failed to kill process");
        }
    }

    let mut child = command
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()
        .unwrap();

    // wait for the process to start. FIXME: process INFO message from STDOUT
    // to start
    std::thread::sleep(std::time::Duration::from_secs(1));

    // cleaner will kill the process when the the function is done
    let _cleaner = Cleaner { p: &mut child };

    f.await;

    Ok(())
}

// Sequentially generates port numbers, which are potentially free. No checks
// are performed to ensure that nobody actually occupies the port.
fn generate_free_port() -> u32 {
    static PORT_NUMBER: Mutex<u32> = Mutex::new(57582);
    let mut guard = PORT_NUMBER.lock().unwrap();
    let result = *guard;
    *guard += 1;
    result
}

// Configures the query-engine binary using given DML string as the schema. Returns command to start
// the query-engine and an HTTP URL to access it.
pub(crate) fn query_engine_cmd(dml: &str) -> (process::Command, String) {
    let mut cmd = std::process::Command::new(query_engine_bin_path());
    // Clear environment to prevent variables set outside of test affect execution.
    cmd.env_clear();

    let port = generate_free_port();
    cmd.env("PRISMA_DML", dml)
        .arg("--port")
        .arg(&port.to_string())
        .arg("-g");

    (cmd, format!("http://0.0.0.0:{}", port))
}

/// Returns the path of the query-engine binary
pub(crate) fn query_engine_bin_path() -> path::PathBuf {
    let name = "query-engine";
    let env_var = format!("CARGO_BIN_EXE_{}", name);
    std::env::var_os(env_var)
        .map(|p| p.into())
        .unwrap_or_else(|| target_dir().join(format!("{}{}", name, env::consts::EXE_SUFFIX)))
}

fn target_dir() -> path::PathBuf {
    env::current_exe()
        .ok()
        .map(|mut path| {
            path.pop();
            if path.ends_with("deps") {
                path.pop();
            }
            path
        })
        .unwrap()
}

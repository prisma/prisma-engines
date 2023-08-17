use super::*;
use once_cell::sync::Lazy;
use query_core::{executor::TransactionManager, QueryExecutor};

#[derive(Debug, Default, Clone, PartialEq)]
pub struct NodeDrivers;

impl ConnectorTagInterface for NodeDrivers {
    fn datamodel_provider(&self) -> &'static str {
        todo!()
    }

    fn datamodel_renderer(&self) -> Box<dyn DatamodelRenderer> {
        todo!()
    }

    fn connection_string(
        &self,
        test_database: &str,
        is_ci: bool,
        is_multi_schema: bool,
        isolation_level: Option<&'static str>,
    ) -> String {
        todo!()
    }

    fn capabilities(&self) -> ConnectorCapabilities {
        todo!()
    }

    fn as_parse_pair(&self) -> (String, Option<String>) {
        todo!()
    }

    fn is_versioned(&self) -> bool {
        todo!()
    }
}

struct NodeProcess {
    process: (std::process::ChildStdin, std::process::ChildStdout),
}

static NODE_PROCESS: Lazy<NodeProcess> = Lazy::new(|| match std::panic::catch_unwind(start_process) {
    Ok(Ok(process)) => process,
    Ok(Err(err)) => todo!(),
    Err(err) => todo!(),
});

fn start_process() -> std::io::Result<NodeProcess> {
    use std::process::{Command, Stdio};

    let env_var =
        std::env::var("NODE_TEST_ADAPTER").map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?;
    let process = Command::new(env_var)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    Ok(NodeProcess {
        process: (process.stdin.unwrap(), process.stdout.unwrap()),
    })
}

impl TransactionManager for NodeProcess {}

impl QueryExecutor for NodeProcess {}

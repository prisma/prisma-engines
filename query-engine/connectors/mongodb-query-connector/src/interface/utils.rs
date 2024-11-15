use std::time::{Duration, Instant};

use mongodb::{
    error::{CommandError, ErrorKind, Result, TRANSIENT_TRANSACTION_ERROR, UNKNOWN_TRANSACTION_COMMIT_RESULT},
    ClientSession,
};

// As suggested by the MongoDB documentation
// https://github.com/mongodb/specifications/blob/master/source/transactions-convenient-api/transactions-convenient-api.rst#pseudo-code
const MAX_TX_TIMEOUT_COMMIT_RETRY_LIMIT: Duration = Duration::from_secs(12);
const TX_RETRY_BACKOFF: Duration = Duration::from_millis(5);

pub async fn commit_with_retry(session: &mut ClientSession) -> Result<()> {
    let timeout = Instant::now();

    while let Err(err) = session.commit_transaction().await {
        // For some reason, MongoDB adds `TRANSIENT_TRANSACTION_ERROR` to errors about aborted
        // transactions. Since transaction will not become less aborted in the future, we handle
        // this case separately.
        let is_aborted = matches!(err.kind.as_ref(), ErrorKind::Command(CommandError { code: 251, .. }));
        let is_in_unknown_state = err.contains_label(UNKNOWN_TRANSACTION_COMMIT_RESULT);
        let is_transient = err.contains_label(TRANSIENT_TRANSACTION_ERROR);
        let is_retryable = !is_aborted && (is_in_unknown_state || is_transient);

        if is_retryable && timeout.elapsed() < MAX_TX_TIMEOUT_COMMIT_RETRY_LIMIT {
            tokio::time::sleep(TX_RETRY_BACKOFF).await;
            continue;
        } else {
            return Err(err);
        }
    }

    Ok(())
}

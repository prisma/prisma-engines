use std::time::{Duration, Instant};

use mongodb::{
    error::{Result, TRANSIENT_TRANSACTION_ERROR, UNKNOWN_TRANSACTION_COMMIT_RESULT},
    ClientSession,
};

// As suggested by the MongoDB documentation
// https://github.com/mongodb/specifications/blob/master/source/transactions-convenient-api/transactions-convenient-api.rst#pseudo-code
const MAX_TX_TIMEOUT_COMMIT_RETRY_LIMIT: Duration = Duration::from_secs(12);
const TX_RETRY_BACKOFF: Duration = Duration::from_millis(5);

pub async fn commit_with_retry(session: &mut ClientSession) -> Result<()> {
    let timeout = Instant::now();

    while let Err(err) = session.commit_transaction().await {
        if (err.contains_label(UNKNOWN_TRANSACTION_COMMIT_RESULT) || err.contains_label(TRANSIENT_TRANSACTION_ERROR))
            && timeout.elapsed() < MAX_TX_TIMEOUT_COMMIT_RETRY_LIMIT
        {
            tokio::time::sleep(TX_RETRY_BACKOFF).await;
            continue;
        } else {
            return Err(err);
        }
    }

    Ok(())
}

use mongodb::{
    error::{Result, TRANSIENT_TRANSACTION_ERROR, UNKNOWN_TRANSACTION_COMMIT_RESULT},
    ClientSession,
};
use std::time::{Duration, Instant};

const MAX_TX_TIMEOUT_COMMIT_RETRY_LIMIT: Duration = Duration::from_secs(12);

pub async fn commit_with_retry(session: &mut ClientSession) -> Result<()> {
    let timeout = Instant::now();

    while let Err(err) = session.commit_transaction().await {
        if (err.contains_label(UNKNOWN_TRANSACTION_COMMIT_RESULT) || err.contains_label(TRANSIENT_TRANSACTION_ERROR))
            && timeout.elapsed() < MAX_TX_TIMEOUT_COMMIT_RETRY_LIMIT
        {
            continue;
        } else {
            return Err(err);
        }
    }

    Ok(())
}

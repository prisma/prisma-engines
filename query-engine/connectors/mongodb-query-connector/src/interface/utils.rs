use std::time::{Duration, Instant};

use mongodb::{
    error::{Result, TRANSIENT_TRANSACTION_ERROR, UNKNOWN_TRANSACTION_COMMIT_RESULT},
    ClientSession,
};
use tokio_retry::strategy;

// As suggested by the MongoDB documentation
// https://github.com/mongodb/specifications/blob/master/source/transactions-convenient-api/transactions-convenient-api.rst#pseudo-code
const MAX_TX_TIMEOUT_COMMIT_RETRY_LIMIT: Duration = Duration::from_secs(12);

pub async fn commit_with_retry(session: &mut ClientSession) -> Result<()> {
    let timeout = Instant::now();

    // backoff strategy: 0ms, 4ms, 8ms, 16ms, 32ms, 64ms, 128ms, 256ms, 512ms, 512ms, ...
    let mut backoff = std::iter::once(Duration::from_secs(0)).chain(
        strategy::ExponentialBackoff::from_millis(2)
            .max_delay(Duration::from_millis(512))
            .factor(2),
    );

    while let Err(err) = session.commit_transaction().await {
        if (err.contains_label(UNKNOWN_TRANSACTION_COMMIT_RESULT) || err.contains_label(TRANSIENT_TRANSACTION_ERROR))
            && timeout.elapsed() < MAX_TX_TIMEOUT_COMMIT_RETRY_LIMIT
        {
            tokio::time::sleep(backoff.next().unwrap()).await;
            continue;
        } else {
            return Err(err);
        }
    }

    Ok(())
}

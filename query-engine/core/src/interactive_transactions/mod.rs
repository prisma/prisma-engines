use derive_more::Display;
use opentelemetry::trace::{SpanId, TraceId};
use serde::Deserialize;

use telemetry::helpers::TraceParent;

mod error;
mod manager;
mod transaction;

pub use error::*;

pub(crate) use manager::*;
pub(crate) use transaction::*;

#[derive(Debug, Clone, Hash, Eq, PartialEq, Deserialize, Display)]
#[display(fmt = "{}", _0)]
pub struct TxId(String);

impl TxId {
    /// This method, as well as `as_span_id`, are intentionally private because it is very easy to
    /// misuse them. Both are used to provide deterministic trace_id and span_id derived from the
    /// transaction id. Both rely on the fact that transaction id is a valid cuid.
    fn as_trace_id(&self) -> TraceId {
        let mut buffer = [0; 16];
        let tx_id = self.0.as_bytes();
        let len = tx_id.len();

        // First 8 bytes after letter 'c' in cuid represent timestamp in milliseconds.
        buffer[0..8].copy_from_slice(&tx_id[1..9]);
        // Last 8 bytes of cuid are totally random.
        buffer[8..].copy_from_slice(&tx_id[len - 8..]);

        TraceId::from_bytes(buffer)
    }

    fn as_span_id(&self) -> SpanId {
        let mut buffer = [0; 8];
        let tx_id = self.0.as_bytes();
        let len = tx_id.len();

        // Last 8 bytes of cuid are totally random.
        buffer[..].copy_from_slice(&tx_id[len - 8..]);

        SpanId::from_bytes(buffer)
    }

    /// Creates new root `TraceParent` that isn't nested under any other spans. Same transaction id
    /// is guaranteed to have traceparent with the same trace_id and span_id.
    pub fn as_traceparent(&self) -> TraceParent {
        TraceParent::new(self.as_trace_id(), self.as_span_id())
    }
}

impl Default for TxId {
    fn default() -> Self {
        #[allow(deprecated)]
        Self(cuid::cuid().unwrap())
    }
}

impl<T> From<T> for TxId
where
    T: Into<String>,
{
    fn from(s: T) -> Self {
        const MINIMUM_TX_ID_LENGTH: usize = 24;

        let contents = s.into();
        // This postcondition is to ensure that the TxId is long enough as to be able to derive
        // a TraceId from it. See `TxTraceExt` trait for more details.
        assert!(
            contents.len() >= MINIMUM_TX_ID_LENGTH,
            "minimum length for a TxId ({}) is {}, but was {}",
            contents,
            MINIMUM_TX_ID_LENGTH,
            contents.len()
        );
        Self(contents)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_txid_into_traceid() {
        let fixture = vec![
            ("clct0q6ma0000rb04768tiqbj", "71366d6130303030373638746971626a"),
            // counter changed, trace id changed:
            ("clct0q6ma0002rb04cpa6zkmx", "71366d6130303032637061367a6b6d78"),
            // fingerprint changed, trace id did not change, as that chunk is ignored:
            ("clct0q6ma00020000cpa6zkmx", "71366d6130303032637061367a6b6d78"),
            // first 5 bytes changed, trace id did not change, as that chunk is ignored:
            ("00000q6ma00020000cpa6zkmx", "71366d6130303032637061367a6b6d78"),
            // 6 th byte changed, trace id changed, as that chunk is part of the lsb of the timestamp
            ("0000006ma00020000cpa6zkmx", "30366d6130303032637061367a6b6d78"),
        ];

        for (txid, expected_trace_id) in fixture {
            let txid: TxId = txid.into();
            let trace_id: opentelemetry::trace::TraceId = txid.as_trace_id();
            assert_eq!(trace_id.to_string(), expected_trace_id);
        }
    }
}

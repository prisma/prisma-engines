use std::collections::HashMap;

use opentelemetry::trace::SpanId;

use crate::helpers::TraceParent;

pub trait TxTraceExt {
    fn as_traceparent(&self) -> TraceParent;
}

impl TxTraceExt for crate::TxId {
    fn as_traceparent(&self) -> TraceParent {
        // in order to convert a TxId (a 48 bytes cuid) into a TraceId (16 bytes), we remove the first byte,
        // (always 'c') and get the next 16 bytes, which are random enough to be used as a trace id.
        // this is a typical cuid: "c-lct0q6ma-0004-rb04-h6en1roa"
        //
        // - first letter is always the same
        // - next 7-8 byte are random a timestamp. There's more entropy in the least significative bytes
        // - next 4 bytes are a counter since the server started
        // - next 4 bytes are a system fingerprint, invariant for the same server instance
        // - least significative 8 bytes. Totally random.
        //
        // We want the most entropic slice of 16 bytes that's deterministicly determined
        let trace_id = {
            let mut buffer = [0; 16];
            let str = self.to_string();
            let tx_id_bytes = str.as_bytes();
            let len = tx_id_bytes.len();

            // bytes [len-20  to len-12): least significative 4 bytes of the timestamp + 4 bytes counter
            for (i, source_idx) in (len - 20..len - 12).enumerate() {
                buffer[i] = tx_id_bytes[source_idx];
            }
            // bytes [len-8 to len):  the random blocks
            for (i, source_idx) in (len - 8..len).enumerate() {
                buffer[i + 8] = tx_id_bytes[source_idx];
            }

            opentelemetry::trace::TraceId::from_bytes(buffer)
        };

        TraceParent::new(trace_id, SpanId::from_hex("0000000000000001").unwrap())
    }
}

// tests for txid into traits
#[cfg(test)]
mod test {
    use super::*;
    use crate::TxId;

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
            let trace_id: opentelemetry::trace::TraceId = txid.as_traceparent().trace_id();
            assert_eq!(trace_id.to_string(), expected_trace_id);
        }
    }
}

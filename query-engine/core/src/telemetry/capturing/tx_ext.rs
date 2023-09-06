use std::collections::HashMap;

pub trait TxTraceExt {
    fn into_trace_id(self) -> opentelemetry::trace::TraceId;
    fn into_trace_context(self) -> opentelemetry::Context;
    fn as_traceparent(&self) -> String;
}

impl TxTraceExt for crate::TxId {
    // in order to convert a TxId (a 17 bytes cuid2) into a TraceId (16 bytes), we remove the first byte,
    // which is a random letter.
    // We want the most entropic slice of 16 bytes that's deterministicly determined: leveraging `cuid2`'s
    // properties, we can just grab the next 16 bytes after the first one.
    fn into_trace_id(self) -> opentelemetry::trace::TraceId {
        let tx_id_str = self.to_string();
        let tx_id_bytes = tx_id_str.as_bytes();

        let mut buffer = [0; 16];

        // Iterate over the tx_id_bytes starting from the second byte (index 1)
        for (i, &byte) in tx_id_bytes.iter().skip(1).enumerate() {
            if i >= 16 {
                break;
            }
            buffer[i] = byte;
        }

        opentelemetry::trace::TraceId::from_bytes(buffer)
    }
    // This is a bit of a hack, but it's the only way to have a default trace span for a whole
    // transaction when no traceparent is propagated from the client.
    //
    // This is done  so we can capture traces happening accross the different queries in a
    // transaction. Otherwise, if a traceparent is not propagated from the client, each query in
    // the transaction will run within a span that has already been generated at the begining of the
    // transaction, and held active in the actor in charge of running the queries. Thus, making
    // impossible to capture traces happening in the individual queries, as they won't be aware of
    // the transaction they are part of.
    //
    // By generating this "fake" traceparent based on the transaction id, we can have a common
    // trace_id for all transaction operations.
    fn into_trace_context(self) -> opentelemetry::Context {
        let extractor: HashMap<String, String> =
            HashMap::from_iter(vec![("traceparent".to_string(), self.as_traceparent())]);
        opentelemetry::global::get_text_map_propagator(|propagator| propagator.extract(&extractor))
    }

    fn as_traceparent(&self) -> String {
        let trace_id = self.clone().into_trace_id();
        format!("00-{trace_id}-0000000000000001-01")
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
            let trace_id: opentelemetry::trace::TraceId = txid.into_trace_id();
            assert_eq!(trace_id.to_string(), expected_trace_id);
        }
    }
}

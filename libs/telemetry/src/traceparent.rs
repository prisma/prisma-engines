use std::{num::ParseIntError, str::FromStr};

use derive_more::Display;
use thiserror::Error;

/// `traceparent` header, as defined by the [W3C Trace Context spec].
///
/// [W3C Trace Context spec]: https://www.w3.org/TR/trace-context/#traceparent-header-field-values
#[derive(Display, Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[display(fmt = "00-{trace_id}-{span_id}-{flags}")]
pub struct TraceParent {
    trace_id: TraceId,
    span_id: SpanId,
    flags: TraceFlags,
}

impl TraceParent {
    pub fn sampled(&self) -> bool {
        self.flags.sampled()
    }

    /// Generates a random `TraceParent`. This is useful in some tests.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new_random() -> Self {
        Self {
            trace_id: TraceId(rand::random()),
            span_id: SpanId(rand::random()),
            flags: TraceFlags::SAMPLED,
        }
    }
}

impl FromStr for TraceParent {
    type Err = ParseTraceParentError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split('-');

        let Some("00") = parts.next() else {
            return Err(ParseTraceParentError::UnsupportedVersion);
        };

        let trace_id = parts.next().ok_or(ParseTraceParentError::MissingTraceId)?;
        let trace_id = trace_id.parse()?;

        let span_id = parts.next().ok_or(ParseTraceParentError::MissingSpanId)?;
        let span_id = span_id.parse()?;

        let flags = parts.next().ok_or(ParseTraceParentError::MissingTraceFlags)?;
        let flags = flags.parse()?;

        Ok(TraceParent {
            trace_id,
            span_id,
            flags,
        })
    }
}

#[derive(Error, Debug)]
pub enum ParseTraceParentError {
    #[error("invalid or unsupported traceparent header version")]
    UnsupportedVersion,

    #[error("cannot parse hex integer: {_0}")]
    InvalidHexValue(#[from] ParseIntError),

    #[error("missing trace ID in traceparent header")]
    MissingTraceId,

    #[error("missing span ID in traceparent header")]
    MissingSpanId,

    #[error("missing trace flags in traceparent header")]
    MissingTraceFlags,
}

macro_rules! parseable_from_hex {
    ($path:path, $ty:ty) => {
        impl FromStr for $path {
            type Err = ParseIntError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                <$ty>::from_str_radix(s, 16).map(Self)
            }
        }
    };
}

#[derive(Display, Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[display(fmt = "{_0:032x}")]
pub struct TraceId(u128);
parseable_from_hex!(TraceId, u128);

#[derive(Display, Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[display(fmt = "{_0:016x}")]
pub struct SpanId(u64);
parseable_from_hex!(SpanId, u64);

#[derive(Display, Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[display(fmt = "{_0:02x}")]
pub struct TraceFlags(u8);
parseable_from_hex!(TraceFlags, u8);

impl TraceFlags {
    #[cfg(not(target_arch = "wasm32"))]
    const SAMPLED: Self = Self(1);

    pub fn sampled(&self) -> bool {
        self.0 & 1 == 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_traceparent() {
        let traceparent = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";
        let result = traceparent.parse::<TraceParent>();
        assert!(result.is_ok());

        let parsed = result.unwrap();
        assert_eq!(parsed.to_string(), traceparent);
    }

    #[test]
    fn test_invalid_version() {
        let traceparent = "01-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";
        let result = traceparent.parse::<TraceParent>();
        assert!(matches!(result, Err(ParseTraceParentError::UnsupportedVersion)));
    }

    #[test]
    fn test_missing_trace_id() {
        let traceparent = "00";
        let result = traceparent.parse::<TraceParent>();
        assert!(matches!(result, Err(ParseTraceParentError::MissingTraceId)));
    }

    #[test]
    fn test_missing_span_id() {
        let traceparent = "00-4bf92f3577b34da6a3ce929d0e0e4736";
        let result = traceparent.parse::<TraceParent>();
        assert!(matches!(result, Err(ParseTraceParentError::MissingSpanId)));
    }

    #[test]
    fn test_missing_flags() {
        let traceparent = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7";
        let result = traceparent.parse::<TraceParent>();
        assert!(matches!(result, Err(ParseTraceParentError::MissingTraceFlags)));
    }

    #[test]
    fn test_invalid_hex_values() {
        let traceparent = "00-xyz-00f067aa0ba902b7-01";
        let result = traceparent.parse::<TraceParent>();
        assert!(matches!(result, Err(ParseTraceParentError::InvalidHexValue(_))));
    }

    #[test]
    fn test_small_values() {
        let traceparent = "00-10-10-1";
        let result = traceparent.parse::<TraceParent>().unwrap();
        assert_eq!(
            result.to_string(),
            "00-00000000000000000000000000000010-0000000000000010-01"
        );
    }
}

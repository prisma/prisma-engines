use std::str::FromStr;

use enumflags2::bitflags;
use serde::Serialize;

/// Log levels in Prisma Client work differently than log levels in `tracing`:
/// enabling a level does not necessarily enable levels above it: in Accelerate,
/// the client specifies the explicit list of log levels it wants to receive per
/// each query. Additionally, Prisma has a `Query` log level. Technically, they
/// aren't really levels in a traditional sense, since they don't have a
/// hierarchy and order relation, but rather categories.
#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[bitflags]
#[repr(u8)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Query,
}

impl From<tracing::Level> for LogLevel {
    fn from(value: tracing::Level) -> Self {
        match value {
            tracing::Level::TRACE => LogLevel::Trace,
            tracing::Level::DEBUG => LogLevel::Debug,
            tracing::Level::INFO => LogLevel::Info,
            tracing::Level::WARN => LogLevel::Warn,
            tracing::Level::ERROR => LogLevel::Error,
        }
    }
}

impl From<&tracing::Level> for LogLevel {
    fn from(value: &tracing::Level) -> Self {
        Self::from(*value)
    }
}

impl FromStr for LogLevel {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "trace" => Ok(LogLevel::Trace),
            "debug" => Ok(LogLevel::Debug),
            "info" => Ok(LogLevel::Info),
            "warn" => Ok(LogLevel::Warn),
            "error" => Ok(LogLevel::Error),
            "query" => Ok(LogLevel::Query),
            _ => Err(()),
        }
    }
}

/// Corresponds to span kinds in OpenTelemetry. Only two kinds are currently
/// used in Prisma, so this enum can be expanded if needed.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub enum SpanKind {
    #[serde(rename = "client")]
    Client,
    #[serde(rename = "internal")]
    Internal,
}

impl FromStr for SpanKind {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "client" => Ok(SpanKind::Client),
            "internal" => Ok(SpanKind::Internal),
            _ => Err(()),
        }
    }
}

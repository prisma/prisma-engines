use std::str::FromStr;

use enumflags2::bitflags;
use serde::Serialize;

#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq)]
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

use std::{
    num::NonZeroU64,
    str::FromStr,
    sync::atomic::{AtomicU64, Ordering},
};

use derive_more::Display;
use serde::{Deserialize, Serialize};

#[derive(Debug, Display, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[display(fmt = "{}", _0)]
#[repr(transparent)]
struct SerializableNonZeroU64(NonZeroU64);

impl SerializableNonZeroU64 {
    pub fn into_u64(self) -> u64 {
        self.0.get()
    }

    pub fn from_u64(value: u64) -> Option<Self> {
        NonZeroU64::new(value).map(Self)
    }
}

impl Serialize for SerializableNonZeroU64 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Serialize as string to preserve full u64 precision in JavaScript. Otherwise values
        // larger than 2^53 - 1 will be parsed as floats on the client side, making it possible for
        // IDs to collide.
        self.to_string().serialize(serializer)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum SerializableNonZeroU64Error {
    #[error("failed to parse string as u64: {0}")]
    ParseError(#[from] std::num::ParseIntError),
    #[error("value must be non-zero")]
    ZeroError,
}

impl FromStr for SerializableNonZeroU64 {
    type Err = SerializableNonZeroU64Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value = s.parse::<u64>()?;
        NonZeroU64::new(value)
            .map(Self)
            .ok_or(SerializableNonZeroU64Error::ZeroError)
    }
}

impl<'de> Deserialize<'de> for SerializableNonZeroU64 {
    fn deserialize<D>(deserializer: D) -> Result<SerializableNonZeroU64, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        let value = value.parse().map_err(serde::de::Error::custom)?;
        Ok(SerializableNonZeroU64(
            NonZeroU64::new(value).ok_or_else(|| serde::de::Error::custom("value must be non-zero"))?,
        ))
    }
}

/// A unique identifier for a span. It maps directly to [`tracing::span::Id`] assigned by
/// [`tracing_subscriber::registry::Registry`].
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[repr(transparent)]
pub struct SpanId(SerializableNonZeroU64);

impl From<&tracing::span::Id> for SpanId {
    fn from(id: &tracing::span::Id) -> Self {
        Self(SerializableNonZeroU64(id.into_non_zero_u64()))
    }
}

impl From<tracing::span::Id> for SpanId {
    fn from(id: tracing::span::Id) -> Self {
        Self::from(&id)
    }
}

/// A unique identifier for an engine trace, representing a tree of spans. These
/// internal traces *do not* correspond to OpenTelemetry trace IDs. One
/// OpenTelemetry trace may contain multiple Prisma Client operations, each of
/// them leading to one or more engine requests. Since engine traces map 1:1 to
/// requests to the engine, we call these trace IDs "request IDs" to
/// disambiguate and avoid confusion.
///
/// We don't use IDs of the root spans themselves for this purpose because span
/// IDs are only guaranteed to be unique among the spans active at the same
/// time. They may be reused after a span is closed, so they are not
/// historically unique. We store the collected spans and events for some short
/// time after the spans are closed until the client requests them, so we need
/// request IDs that are guaranteed to be unique for a very long period of time
/// (although they still don't necessarily have to be unique for the whole
/// lifetime of the process).
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[repr(transparent)]
pub struct RequestId(SerializableNonZeroU64);

impl RequestId {
    pub fn next() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);

        let mut id = 0;
        while id == 0 {
            id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        }

        Self(SerializableNonZeroU64(NonZeroU64::new(id).unwrap()))
    }

    pub fn into_u64(self) -> u64 {
        self.0.into_u64()
    }

    pub fn from_u64(value: u64) -> Option<Self> {
        SerializableNonZeroU64::from_u64(value).map(Self)
    }
}

impl Default for RequestId {
    fn default() -> Self {
        Self::next()
    }
}

impl FromStr for RequestId {
    type Err = SerializableNonZeroU64Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        SerializableNonZeroU64::from_str(s).map(Self)
    }
}

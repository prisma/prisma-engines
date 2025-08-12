use alloc::string::ToString;

use serde::{Serialize, ser::SerializeMap};
use serde_json::json;

use crate::{PrismaValue, encode_bytes};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TaggedPrismaValue<'a>(&'a PrismaValue);

impl Serialize for TaggedPrismaValue<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self.0 {
            PrismaValue::Bytes(bytes) => {
                serializer.collect_map([("prisma__type", "bytes"), ("prisma__value", &encode_bytes(bytes))])
            }
            PrismaValue::GeneratorCall {
                name,
                args,
                return_type,
            } => {
                let mut map = serializer.serialize_map(Some(2))?;
                map.serialize_entry("prisma__type", "generatorCall")?;
                map.serialize_entry(
                    "prisma__value",
                    &json!({
                        "name": name,
                        "args": TaggedPrismaValueSliceIter::new(args.iter()),
                        "returnType": return_type,
                    }),
                )?;
                map.end()
            }
            PrismaValue::BigInt(i) => {
                serializer.collect_map([("prisma__type", "bigint"), ("prisma__value", &i.to_string())])
            }
            PrismaValue::List(items) => serializer.collect_seq(items.iter().map(TaggedPrismaValue)),
            PrismaValue::Object(items) => serializer.collect_map(items.iter().map(|(k, v)| (k, TaggedPrismaValue(v)))),
            other => other.serialize(serializer),
        }
    }
}

impl<'a> From<&'a PrismaValue> for TaggedPrismaValue<'a> {
    fn from(value: &'a PrismaValue) -> Self {
        TaggedPrismaValue(value)
    }
}

struct TaggedPrismaValueSliceIter<'a> {
    iter: core::slice::Iter<'a, PrismaValue>,
}

impl<'a> TaggedPrismaValueSliceIter<'a> {
    fn new(iter: core::slice::Iter<'a, PrismaValue>) -> Self {
        Self { iter }
    }
}

impl Serialize for TaggedPrismaValueSliceIter<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_seq(self.iter.clone())
    }
}

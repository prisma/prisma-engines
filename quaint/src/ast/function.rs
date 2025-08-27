mod aggregate_to_string;
mod average;
mod coalesce;
mod concat;
mod count;
mod json_array_agg;
mod json_build_obj;
mod json_extract;
mod json_extract_array;
mod json_unquote;
mod lower;
mod maximum;
mod minimum;
mod now;
mod row_number;
mod row_to_json;
mod search;
mod sum;
mod upper;

mod uuid;

pub use aggregate_to_string::*;
pub use average::*;
pub use coalesce::*;
pub use concat::*;
pub use count::*;
pub use json_array_agg::*;
pub use json_build_obj::*;
pub use json_extract::*;
pub(crate) use json_extract_array::*;
pub use json_unquote::*;
pub use lower::*;
pub use maximum::*;
pub use minimum::*;
pub use now::*;
pub use row_number::*;
pub use row_to_json::*;
pub use search::*;
pub use sum::*;
pub use upper::*;

pub use self::uuid::*;

use super::{Aliasable, Expression};
use std::borrow::Cow;

/// A database function definition
#[derive(Debug, Clone, PartialEq)]
pub struct Function<'a> {
    pub(crate) typ_: FunctionType<'a>,
    pub(crate) alias: Option<Cow<'a, str>>,
}

impl Function<'_> {
    pub fn returns_json(&self) -> bool {
        matches!(
            self.typ_,
            FunctionType::RowToJson(_)
                | FunctionType::JsonExtract(_)
                | FunctionType::JsonExtractLastArrayElem(_)
                | FunctionType::JsonExtractFirstArrayElem(_)
        )
    }
}

/// A database function type.
/// Not every function is supported by every database.
/// TODO: Use `cfg` compilation flags to enable/disable functions based on the database family.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum FunctionType<'a> {
    RowToJson(RowToJson<'a>),
    RowNumber(RowNumber<'a>),
    Count(Count<'a>),
    AggregateToString(AggregateToString<'a>),
    Average(Average<'a>),
    Sum(Sum<'a>),
    Lower(Lower<'a>),
    Upper(Upper<'a>),
    Minimum(Minimum<'a>),
    Maximum(Maximum<'a>),
    Coalesce(Coalesce<'a>),
    Concat(Concat<'a>),
    JsonExtract(JsonExtract<'a>),
    JsonExtractLastArrayElem(JsonExtractLastArrayElem<'a>),
    JsonExtractFirstArrayElem(JsonExtractFirstArrayElem<'a>),
    JsonUnquote(JsonUnquote<'a>),
    JsonArrayAgg(JsonArrayAgg<'a>),
    JsonBuildObject(JsonBuildObject<'a>),
    TextSearch(TextSearch<'a>),
    TextSearchRelevance(TextSearchRelevance<'a>),
    UuidToBin,
    UuidToBinSwapped,
    Uuid,
    Now,
}

impl<'a> Aliasable<'a> for Function<'a> {
    type Target = Function<'a>;

    fn alias<T>(mut self, alias: T) -> Self::Target
    where
        T: Into<Cow<'a, str>>,
    {
        self.alias = Some(alias.into());
        self
    }
}

function!(RowToJson);

function!(JsonExtract);

function!(JsonExtractLastArrayElem);

function!(JsonExtractFirstArrayElem);

function!(JsonUnquote);

function!(TextSearch);

function!(TextSearchRelevance);

function!(JsonArrayAgg);

function!(JsonBuildObject);

function!(
    RowNumber,
    Count,
    AggregateToString,
    Average,
    Sum,
    Lower,
    Upper,
    Minimum,
    Maximum,
    Coalesce,
    Concat
);

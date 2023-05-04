use psl::parser_database::walkers;
use sql_schema_describer as sql;
use std::borrow::Cow;

use super::{IntrospectionPair, ScalarFieldPair};

/// Pairing PSL index field to field in database index definition.
/// Both values are optional, due to in some cases we plainly just copy
/// the PSL argument to the rendered data model.
///
/// This happens with views, where we need at least one unique
/// field in the view definition, but the database does not
/// hold constraints on views.
pub(crate) type IndexFieldPair<'a> =
    IntrospectionPair<'a, Option<walkers::ScalarFieldWalker<'a>>, Option<sql::IndexColumnWalker<'a>>>;

pub(crate) enum IndexOps<'a> {
    Managed(&'a str),
    Raw(&'a str),
}

impl<'a> IndexFieldPair<'a> {
    /// The name of the field (as used in Prisma).
    pub(crate) fn name(self) -> Cow<'a, str> {
        match self.field() {
            Some(field) => field.name(),
            None => Cow::Borrowed(self.previous.unwrap().name()),
        }
    }

    /// The ordering of the column in the database. Returns a value if
    /// non-default.
    pub(crate) fn sort_order(self) -> Option<&'static str> {
        match self.next {
            Some(next) => next
                .sort_order()
                .filter(|so| matches!(so, sql::SQLSortOrder::Desc))
                .map(|_| "Desc"),
            None => None,
        }
    }

    /// A MySQL specific length definition for the indexed column.
    pub(crate) fn length(self) -> Option<u32> {
        self.next.and_then(|next| next.length())
    }

    /// True, if we _add_ an index with non-default null position.
    pub(crate) fn adds_non_default_null_position(self) -> bool {
        if self.previous.is_some() {
            return false;
        }

        match self.next {
            Some(next) => self.context.flavour.uses_non_default_null_position(self.context, next),
            None => false,
        }
    }

    /// A PostgreSQL specific operator class for the indexed column.
    #[cfg(feature = "postgresql")]
    pub(crate) fn opclass(self) -> Option<IndexOps<'a>> {
        if !self.context.sql_family.is_postgres() {
            return None;
        }

        let ext: &sql::postgres::PostgresSchemaExt = self.context.sql_schema.downcast_connector_data();

        let next = match self.next {
            Some(next) => next,
            None => return None,
        };

        let opclass = match ext.get_opclass(next.id) {
            Some(opclass) => opclass,
            None => return None,
        };

        match &opclass.kind {
            _ if opclass.is_default => None,
            sql::postgres::SQLOperatorClassKind::InetOps => Some(IndexOps::Managed("InetOps")),
            sql::postgres::SQLOperatorClassKind::JsonbOps => Some(IndexOps::Managed("JsonbOps")),
            sql::postgres::SQLOperatorClassKind::JsonbPathOps => Some(IndexOps::Managed("JsonbPathOps")),
            sql::postgres::SQLOperatorClassKind::ArrayOps => Some(IndexOps::Managed("ArrayOps")),
            sql::postgres::SQLOperatorClassKind::TextOps => Some(IndexOps::Managed("TextOps")),
            sql::postgres::SQLOperatorClassKind::BitMinMaxOps => Some(IndexOps::Managed("BitMinMaxOps")),
            sql::postgres::SQLOperatorClassKind::VarBitMinMaxOps => Some(IndexOps::Managed("VarBitMinMaxOps")),
            sql::postgres::SQLOperatorClassKind::BpcharBloomOps => Some(IndexOps::Managed("BpcharBloomOps")),
            sql::postgres::SQLOperatorClassKind::BpcharMinMaxOps => Some(IndexOps::Managed("BpcharMinMaxOps")),
            sql::postgres::SQLOperatorClassKind::ByteaBloomOps => Some(IndexOps::Managed("ByteaBloomOps")),
            sql::postgres::SQLOperatorClassKind::ByteaMinMaxOps => Some(IndexOps::Managed("ByteaMinMaxOps")),
            sql::postgres::SQLOperatorClassKind::DateBloomOps => Some(IndexOps::Managed("DateBloomOps")),
            sql::postgres::SQLOperatorClassKind::DateMinMaxOps => Some(IndexOps::Managed("DateMinMaxOps")),
            sql::postgres::SQLOperatorClassKind::DateMinMaxMultiOps => Some(IndexOps::Managed("DateMinMaxMultiOps")),
            sql::postgres::SQLOperatorClassKind::Float4BloomOps => Some(IndexOps::Managed("Float4BloomOps")),
            sql::postgres::SQLOperatorClassKind::Float4MinMaxOps => Some(IndexOps::Managed("Float4MinMaxOps")),
            sql::postgres::SQLOperatorClassKind::Float4MinMaxMultiOps => {
                Some(IndexOps::Managed("Float4MinMaxMultiOps"))
            }
            sql::postgres::SQLOperatorClassKind::Float8BloomOps => Some(IndexOps::Managed("Float8BloomOps")),
            sql::postgres::SQLOperatorClassKind::Float8MinMaxOps => Some(IndexOps::Managed("Float8MinMaxOps")),
            sql::postgres::SQLOperatorClassKind::Float8MinMaxMultiOps => {
                Some(IndexOps::Managed("Float8MinMaxMultiOps"))
            }
            sql::postgres::SQLOperatorClassKind::InetInclusionOps => Some(IndexOps::Managed("InetInclusionOps")),
            sql::postgres::SQLOperatorClassKind::InetBloomOps => Some(IndexOps::Managed("InetBloomOps")),
            sql::postgres::SQLOperatorClassKind::InetMinMaxOps => Some(IndexOps::Managed("InetMinMaxOps")),
            sql::postgres::SQLOperatorClassKind::InetMinMaxMultiOps => Some(IndexOps::Managed("InetMinMaxMultiOps")),
            sql::postgres::SQLOperatorClassKind::Int2BloomOps => Some(IndexOps::Managed("Int2BloomOps")),
            sql::postgres::SQLOperatorClassKind::Int2MinMaxOps => Some(IndexOps::Managed("Int2MinMaxOps")),
            sql::postgres::SQLOperatorClassKind::Int2MinMaxMultiOps => Some(IndexOps::Managed("Int2MinMaxMultiOps")),
            sql::postgres::SQLOperatorClassKind::Int4BloomOps => Some(IndexOps::Managed("Int4BloomOps")),
            sql::postgres::SQLOperatorClassKind::Int4MinMaxOps => Some(IndexOps::Managed("Int4MinMaxOps")),
            sql::postgres::SQLOperatorClassKind::Int4MinMaxMultiOps => Some(IndexOps::Managed("Int4MinMaxMultiOps")),
            sql::postgres::SQLOperatorClassKind::Int8BloomOps => Some(IndexOps::Managed("Int8BloomOps")),
            sql::postgres::SQLOperatorClassKind::Int8MinMaxOps => Some(IndexOps::Managed("Int8MinMaxOps")),
            sql::postgres::SQLOperatorClassKind::Int8MinMaxMultiOps => Some(IndexOps::Managed("Int8MinMaxMultiOps")),
            sql::postgres::SQLOperatorClassKind::NumericBloomOps => Some(IndexOps::Managed("NumericBloomOps")),
            sql::postgres::SQLOperatorClassKind::NumericMinMaxOps => Some(IndexOps::Managed("NumericMinMaxOps")),
            sql::postgres::SQLOperatorClassKind::NumericMinMaxMultiOps => {
                Some(IndexOps::Managed("NumericMinMaxMultiOps"))
            }
            sql::postgres::SQLOperatorClassKind::OidBloomOps => Some(IndexOps::Managed("OidBloomOps")),
            sql::postgres::SQLOperatorClassKind::OidMinMaxOps => Some(IndexOps::Managed("OidMinMaxOps")),
            sql::postgres::SQLOperatorClassKind::OidMinMaxMultiOps => Some(IndexOps::Managed("OidMinMaxMultiOps")),
            sql::postgres::SQLOperatorClassKind::TextBloomOps => Some(IndexOps::Managed("TextBloomOps")),
            sql::postgres::SQLOperatorClassKind::TextMinMaxOps => Some(IndexOps::Managed("TextMinMaxOps")),
            sql::postgres::SQLOperatorClassKind::TimestampBloomOps => Some(IndexOps::Managed("TimestampBloomOps")),
            sql::postgres::SQLOperatorClassKind::TimestampMinMaxOps => Some(IndexOps::Managed("TimestampMinMaxOps")),
            sql::postgres::SQLOperatorClassKind::TimestampMinMaxMultiOps => {
                Some(IndexOps::Managed("TimestampMinMaxMultiOps"))
            }
            sql::postgres::SQLOperatorClassKind::TimestampTzBloomOps => Some(IndexOps::Managed("TimestampTzBloomOps")),
            sql::postgres::SQLOperatorClassKind::TimestampTzMinMaxOps => {
                Some(IndexOps::Managed("TimestampTzMinMaxOps"))
            }
            sql::postgres::SQLOperatorClassKind::TimestampTzMinMaxMultiOps => {
                Some(IndexOps::Managed("TimestampTzMinMaxMultiOps"))
            }
            sql::postgres::SQLOperatorClassKind::TimeBloomOps => Some(IndexOps::Managed("TimeBloomOps")),
            sql::postgres::SQLOperatorClassKind::TimeMinMaxOps => Some(IndexOps::Managed("TimeMinMaxOps")),
            sql::postgres::SQLOperatorClassKind::TimeMinMaxMultiOps => Some(IndexOps::Managed("TimeMinMaxMultiOps")),
            sql::postgres::SQLOperatorClassKind::TimeTzBloomOps => Some(IndexOps::Managed("TimeTzBloomOps")),
            sql::postgres::SQLOperatorClassKind::TimeTzMinMaxOps => Some(IndexOps::Managed("TimeTzMinMaxOps")),
            sql::postgres::SQLOperatorClassKind::TimeTzMinMaxMultiOps => {
                Some(IndexOps::Managed("TimeTzMinMaxMultiOps"))
            }
            sql::postgres::SQLOperatorClassKind::UuidBloomOps => Some(IndexOps::Managed("UuidBloomOps")),
            sql::postgres::SQLOperatorClassKind::UuidMinMaxOps => Some(IndexOps::Managed("UuidMinMaxOps")),
            sql::postgres::SQLOperatorClassKind::UuidMinMaxMultiOps => Some(IndexOps::Managed("UuidMinMaxMultiOps")),
            sql::postgres::SQLOperatorClassKind::Raw(ref c) => Some(IndexOps::Raw(c)),
        }
    }

    fn field(self) -> Option<ScalarFieldPair<'a>> {
        let next = match self.next {
            Some(next) => next,
            None => return None,
        };

        let previous = self.context.existing_table_scalar_field(next.as_column().id);
        let next = next.as_column();

        Some(IntrospectionPair::new(self.context, previous, next.coarsen()))
    }
}

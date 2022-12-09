use std::{borrow::Cow, fmt};

use psl::{
    builtin_connectors::{MySqlType, PostgresType},
    datamodel_connector::constraint_names::ConstraintNames,
    dml,
    parser_database::walkers,
};
use sql::postgres::PostgresSchemaExt;
use sql_schema_describer as sql;

use super::Pair;

pub(crate) type DefaultValuePair<'a> = Pair<'a, walkers::DefaultValueWalker<'a>, sql::ColumnWalker<'a>>;

pub(crate) enum DefaultKind<'a> {
    Sequence(&'a sql::postgres::Sequence),
    DbGenerated(Option<&'a str>),
    Autoincrement,
    Uuid,
    Cuid,
    Prisma1Uuid,
    Prisma1Cuid,
    Now,
    String(&'a str),
    StringList(Vec<&'a str>),
    EnumVariant(Cow<'a, str>),
    Constant(&'a dyn fmt::Display),
    ConstantList(Vec<&'a dyn fmt::Display>),
    Bytes(&'a [u8]),
    BytesList(Vec<&'a [u8]>),
}

impl<'a> DefaultValuePair<'a> {
    /// The default value, if defined either in the database or PSL.
    pub(crate) fn kind(self) -> Option<DefaultKind<'a>> {
        let sql_kind = self.next.default().map(|d| d.kind());
        let family = self.next.column_type_family();

        match (sql_kind, family) {
            (Some(sql::DefaultKind::Sequence(name)), _) if self.context.is_cockroach() => {
                let connector_data: &PostgresSchemaExt = self.context.schema.downcast_connector_data();

                let sequence_idx = connector_data
                    .sequences
                    .binary_search_by_key(&name, |s| &s.name)
                    .unwrap();

                Some(DefaultKind::Sequence(&connector_data.sequences[sequence_idx]))
            }
            (_, sql::ColumnTypeFamily::Int | sql::ColumnTypeFamily::BigInt) if self.next.is_autoincrement() => {
                Some(DefaultKind::Autoincrement)
            }
            (Some(sql::DefaultKind::Sequence(_)), _) => Some(DefaultKind::Autoincrement),
            (Some(sql::DefaultKind::UniqueRowid), _) => Some(DefaultKind::Autoincrement),

            (Some(sql::DefaultKind::DbGenerated(default_string)), _) => {
                Some(DefaultKind::DbGenerated(default_string.as_deref()))
            }

            (Some(sql::DefaultKind::Now), sql::ColumnTypeFamily::DateTime) => Some(DefaultKind::Now),

            (Some(sql::DefaultKind::Value(dml::PrismaValue::Null)), _) => Some(DefaultKind::Constant(&"null")),
            (Some(sql::DefaultKind::Value(dml::PrismaValue::String(val))), _) => Some(DefaultKind::String(val)),
            (Some(sql::DefaultKind::Value(dml::PrismaValue::Json(val))), _) => Some(DefaultKind::String(val)),
            (Some(sql::DefaultKind::Value(dml::PrismaValue::Xml(val))), _) => Some(DefaultKind::String(val)),

            (Some(sql::DefaultKind::Value(dml::PrismaValue::Boolean(val))), _) => Some(DefaultKind::Constant(val)),
            (Some(sql::DefaultKind::Value(dml::PrismaValue::Enum(variant))), sql::ColumnTypeFamily::Enum(enum_id)) => {
                let variant = self
                    .context
                    .schema
                    .walk(*enum_id)
                    .variants()
                    .find(|v| v.name() == variant)
                    .unwrap();

                let variant_name = self.context.enum_variant_name(variant.id);

                if !variant_name.prisma_name().is_empty() {
                    Some(DefaultKind::EnumVariant(variant_name.prisma_name()))
                } else {
                    Some(DefaultKind::DbGenerated(variant_name.mapped_name()))
                }
            }
            (Some(sql::DefaultKind::Value(dml::PrismaValue::Int(val))), _) => Some(DefaultKind::Constant(val)),
            (Some(sql::DefaultKind::Value(dml::PrismaValue::Uuid(val))), _) => Some(DefaultKind::Constant(val)),
            (Some(sql::DefaultKind::Value(dml::PrismaValue::DateTime(val))), _) => Some(DefaultKind::Constant(val)),
            (Some(sql::DefaultKind::Value(dml::PrismaValue::Float(val))), _) => Some(DefaultKind::Constant(val)),
            (Some(sql::DefaultKind::Value(dml::PrismaValue::BigInt(val))), _) => Some(DefaultKind::Constant(val)),

            (Some(sql::DefaultKind::Value(dml::PrismaValue::Bytes(val))), _) => Some(DefaultKind::Bytes(val)),

            (Some(sql::DefaultKind::Value(dml::PrismaValue::List(vals))), _) => match vals.first() {
                None => Some(DefaultKind::ConstantList(Vec::new())),
                Some(dml::PrismaValue::String(_) | dml::PrismaValue::Xml(_) | dml::PrismaValue::Json(_)) => {
                    let vals = vals.iter().filter_map(|val| val.as_string()).collect();
                    Some(DefaultKind::StringList(vals))
                }
                Some(
                    dml::PrismaValue::Boolean(_)
                    | dml::PrismaValue::Enum(_)
                    | dml::PrismaValue::Int(_)
                    | dml::PrismaValue::Uuid(_)
                    | dml::PrismaValue::DateTime(_)
                    | dml::PrismaValue::Float(_)
                    | dml::PrismaValue::BigInt(_),
                ) => {
                    let vals = vals.iter().map(|val| val as &'a dyn fmt::Display).collect();
                    Some(DefaultKind::ConstantList(vals))
                }
                Some(dml::PrismaValue::Null) => {
                    let vals = vals.iter().map(|_| &"null" as &'a dyn fmt::Display).collect();
                    Some(DefaultKind::ConstantList(vals))
                }
                Some(dml::PrismaValue::Bytes(_)) => {
                    let vals = vals.iter().filter_map(|val| val.as_bytes()).collect();
                    Some(DefaultKind::BytesList(vals))
                }
                _ => unreachable!(),
            },

            (None, sql::ColumnTypeFamily::String) => match self.previous {
                Some(previous) if previous.is_cuid() => Some(DefaultKind::Cuid),
                Some(previous) if previous.is_uuid() => Some(DefaultKind::Uuid),
                None if self.context.version.is_prisma1() && self.context.sql_family.is_postgres() => {
                    let native_type: &PostgresType = self.next.column_type().native_type.as_ref()?.downcast_ref();

                    if native_type == &PostgresType::VarChar(Some(25)) {
                        Some(DefaultKind::Prisma1Cuid)
                    } else if native_type == &PostgresType::VarChar(Some(36)) {
                        Some(DefaultKind::Prisma1Uuid)
                    } else {
                        None
                    }
                }
                None if self.context.version.is_prisma1() && self.context.sql_family.is_mysql() => {
                    let native_type: &MySqlType = self.next.column_type().native_type.as_ref()?.downcast_ref();

                    if native_type == &MySqlType::Char(25) {
                        Some(DefaultKind::Prisma1Cuid)
                    } else if native_type == &MySqlType::Char(36) {
                        Some(DefaultKind::Prisma1Uuid)
                    } else {
                        None
                    }
                }
                _ => None,
            },

            _ => None,
        }
    }

    /// The constraint name, if the database uses them for defaults
    /// and it is non-default.
    pub(crate) fn mapped_name(self) -> Option<&'a str> {
        match self.next.default() {
            Some(def) => def.constraint_name().filter(move |name| name != &self.default_name()),
            None => None,
        }
    }

    fn default_name(self) -> String {
        ConstraintNames::default_name(
            self.next.table().name(),
            self.next.name(),
            self.context.active_connector(),
        )
    }
}

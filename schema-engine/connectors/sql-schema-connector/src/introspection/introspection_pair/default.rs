use either::Either;
use prisma_value::PrismaValue;
use psl::{datamodel_connector::constraint_names::ConstraintNames, parser_database::walkers};
use sql::postgres::PostgresSchemaExt;
use sql_schema_describer as sql;
use std::{borrow::Cow, fmt};

use super::IntrospectionPair;

pub(crate) type DefaultValuePair<'a> =
    IntrospectionPair<'a, Option<walkers::DefaultValueWalker<'a>>, sql::ColumnWalker<'a>>;

pub(crate) enum DefaultKind<'a> {
    Sequence(&'a sql::postgres::Sequence, sql::ColumnTypeFamily),
    DbGenerated(Option<&'a str>),
    Autoincrement,
    Uuid(Option<u8>),
    Cuid(Option<u8>),
    Ulid,
    Nanoid(Option<u8>),
    Now,
    String(&'a str),
    StringList(Vec<&'a str>),
    EnumVariant(Cow<'a, str>),
    EnumVariantList(Vec<Cow<'a, str>>),
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
                let connector_data: &PostgresSchemaExt = self.context.sql_schema.downcast_connector_data();

                let sequence_idx = connector_data
                    .sequences
                    .binary_search_by_key(&name, |s| &s.name)
                    .unwrap();

                Some(DefaultKind::Sequence(
                    &connector_data.sequences[sequence_idx],
                    family.clone(),
                ))
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

            (Some(sql::DefaultKind::Value(PrismaValue::Null)), _) => Some(DefaultKind::Constant(&"null")),
            (Some(sql::DefaultKind::Value(PrismaValue::String(val))), _) => Some(DefaultKind::String(val)),
            (Some(sql::DefaultKind::Value(PrismaValue::Json(val))), _) => Some(DefaultKind::String(val)),

            (Some(sql::DefaultKind::Value(PrismaValue::Boolean(val))), _) => Some(DefaultKind::Constant(val)),
            (Some(sql::DefaultKind::Value(PrismaValue::Enum(variant))), sql::ColumnTypeFamily::Enum(enum_id)) => {
                let variant = self
                    .context
                    .sql_schema
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
            (Some(sql::DefaultKind::Value(PrismaValue::Int(val))), _) => Some(DefaultKind::Constant(val)),
            (Some(sql::DefaultKind::Value(PrismaValue::Uuid(val))), _) => Some(DefaultKind::Constant(val)),
            (Some(sql::DefaultKind::Value(PrismaValue::DateTime(val))), _) => Some(DefaultKind::Constant(val)),
            (Some(sql::DefaultKind::Value(PrismaValue::Float(val))), _) => Some(DefaultKind::Constant(val)),
            (Some(sql::DefaultKind::Value(PrismaValue::BigInt(val))), _) => Some(DefaultKind::Constant(val)),

            (Some(sql::DefaultKind::Value(PrismaValue::Bytes(val))), _) => Some(DefaultKind::Bytes(val)),

            // Special handling for enum arrays: map database values to Prisma variant names
            (Some(sql::DefaultKind::Value(PrismaValue::List(vals))), sql::ColumnTypeFamily::Enum(enum_id)) => {
                if vals.is_empty() {
                    return Some(DefaultKind::EnumVariantList(Vec::new()));
                }

                let enum_walker = self.context.sql_schema.walk(*enum_id);
                let mut out: Vec<Cow<'a, str>> = Vec::with_capacity(vals.len());

                for val in vals {
                    let PrismaValue::Enum(db_val) = val else {
                        return None;
                    };

                    let variant = enum_walker.variants().find(|v| v.name() == db_val).unwrap();

                    let variant_name = self.context.enum_variant_name(variant.id);

                    if variant_name.prisma_name().is_empty() {
                        return Some(DefaultKind::DbGenerated(variant_name.mapped_name()));
                    }

                    out.push(variant_name.prisma_name());
                }

                Some(DefaultKind::EnumVariantList(out))
            }

            (Some(sql::DefaultKind::Value(PrismaValue::List(vals))), _) => match vals.first() {
                None => Some(DefaultKind::ConstantList(Vec::new())),
                Some(PrismaValue::String(_) | PrismaValue::Json(_)) => {
                    let vals = vals.iter().filter_map(|val| val.as_string()).collect();
                    Some(DefaultKind::StringList(vals))
                }
                Some(
                    PrismaValue::Boolean(_)
                    | PrismaValue::Enum(_)
                    | PrismaValue::Int(_)
                    | PrismaValue::Uuid(_)
                    | PrismaValue::DateTime(_)
                    | PrismaValue::Float(_)
                    | PrismaValue::BigInt(_),
                ) => {
                    let vals = vals.iter().map(|val| val as &'a dyn fmt::Display).collect();
                    Some(DefaultKind::ConstantList(vals))
                }
                Some(PrismaValue::Null) => {
                    let vals = vals.iter().map(|_| &"null" as &'a dyn fmt::Display).collect();
                    Some(DefaultKind::ConstantList(vals))
                }
                Some(PrismaValue::Bytes(_)) => {
                    let vals = vals.iter().filter_map(|val| val.as_bytes()).collect();
                    Some(DefaultKind::BytesList(vals))
                }
                _ => unreachable!(),
            },

            (None, sql::ColumnTypeFamily::String | sql::ColumnTypeFamily::Uuid) => match self.previous {
                Some(previous) if previous.is_ulid() => Some(DefaultKind::Ulid),
                Some(previous) if previous.is_cuid() => {
                    let version = previous.value().as_function().and_then(|(_, args, _)| {
                        args.arguments
                            .first()
                            .map(|arg| arg.value.as_numeric_value().unwrap().0.parse::<u8>().unwrap())
                    });

                    Some(DefaultKind::Cuid(version))
                }
                Some(previous) if previous.is_uuid() => {
                    let version = previous.value().as_function().and_then(|(_, args, _)| {
                        args.arguments
                            .first()
                            .map(|arg| arg.value.as_numeric_value().unwrap().0.parse::<u8>().unwrap())
                    });

                    Some(DefaultKind::Uuid(version))
                }
                Some(previous) if previous.is_nanoid() => {
                    let length = previous.value().as_function().and_then(|(_, args, _)| {
                        args.arguments
                            .first()
                            .map(|arg| arg.value.as_numeric_value().unwrap().0.parse::<u8>().unwrap())
                    });

                    Some(DefaultKind::Nanoid(length))
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
        let container_name = match self.next.refine() {
            Either::Left(col) => col.table().name(),
            Either::Right(col) => col.view().name(),
        };

        ConstraintNames::default_name(container_name, self.next.name(), self.context.active_connector())
    }
}

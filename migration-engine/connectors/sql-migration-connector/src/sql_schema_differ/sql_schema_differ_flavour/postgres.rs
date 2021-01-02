use super::SqlSchemaDifferFlavour;
use crate::flavour::SqlFlavour;
use crate::{
    flavour::PostgresFlavour,
    pair::Pair,
    sql_migration::AlterEnum,
    sql_schema_differ::column::{ColumnDiffer, ColumnTypeChange},
    sql_schema_differ::SqlSchemaDiffer,
};
use migration_connector::MigrationFeature;
use native_types::PostgresType;
use once_cell::sync::Lazy;
use regex::RegexSet;
use sql_schema_describer::{walkers::IndexWalker, ColumnTypeFamily};

/// The maximum length of postgres identifiers, in bytes.
///
/// Reference: https://www.postgresql.org/docs/12/limits.html
const POSTGRES_IDENTIFIER_SIZE_LIMIT: usize = 63;

impl SqlSchemaDifferFlavour for PostgresFlavour {
    fn alter_enums(&self, differ: &SqlSchemaDiffer<'_>) -> Vec<AlterEnum> {
        differ
            .enum_pairs()
            .filter_map(|differ| {
                let step = AlterEnum {
                    index: differ.enums.as_ref().map(|e| e.enum_index()),
                    created_variants: differ.created_values().map(String::from).collect(),
                    dropped_variants: differ.dropped_values().map(String::from).collect(),
                };

                if step.is_empty() {
                    None
                } else {
                    Some(step)
                }
            })
            .collect()
    }

    fn index_should_be_renamed(&self, pair: &Pair<IndexWalker<'_>>) -> bool {
        // Implements correct comparison for truncated index names.
        let (previous_name, next_name) = pair.as_ref().map(|idx| idx.name()).into_tuple();

        if previous_name.len() == POSTGRES_IDENTIFIER_SIZE_LIMIT && next_name.len() > POSTGRES_IDENTIFIER_SIZE_LIMIT {
            previous_name[0..POSTGRES_IDENTIFIER_SIZE_LIMIT] != next_name[0..POSTGRES_IDENTIFIER_SIZE_LIMIT]
        } else {
            previous_name != next_name
        }
    }

    fn table_should_be_ignored(&self, table_name: &str) -> bool {
        static POSTGRES_IGNORED_TABLES: Lazy<RegexSet> = Lazy::new(|| {
            RegexSet::new(&[
                // PostGIS. Reference: https://postgis.net/docs/manual-1.4/ch04.html#id418599
                "(?i)^spatial_ref_sys$",
                "(?i)^geometry_columns$",
            ])
            .unwrap()
        });

        POSTGRES_IGNORED_TABLES.is_match(table_name)
    }

    fn column_type_change(&self, differ: &ColumnDiffer<'_>) -> Option<ColumnTypeChange> {
        let native_types_enabled = self.features().contains(MigrationFeature::NativeTypes);
        let previous_family = differ.previous.column_type_family();
        let next_family = differ.next.column_type_family();
        let previous_type: Option<PostgresType> = differ.previous.column_native_type();
        let next_type: Option<PostgresType> = differ.next.column_native_type();
        let from_list_to_scalar = differ.previous.arity().is_list() && !differ.next.arity().is_list();
        let from_scalar_to_list = !differ.previous.arity().is_list() && differ.next.arity().is_list();

        if !native_types_enabled {
            match (previous_family, next_family) {
                (_, ColumnTypeFamily::String) if from_list_to_scalar => Some(ColumnTypeChange::SafeCast),
                (_, _) if from_list_to_scalar => Some(ColumnTypeChange::NotCastable),
                (ColumnTypeFamily::Decimal, ColumnTypeFamily::Decimal)
                | (ColumnTypeFamily::Float, ColumnTypeFamily::Float)
                | (ColumnTypeFamily::Decimal, ColumnTypeFamily::Float)
                | (ColumnTypeFamily::Float, ColumnTypeFamily::Decimal)
                | (ColumnTypeFamily::Binary, ColumnTypeFamily::Binary)
                    if from_scalar_to_list =>
                {
                    Some(ColumnTypeChange::NotCastable)
                }
                (previous, next) => family_change_riskyness(previous, next),
            }
        } else {
            native_type_change_riskyness(previous_type.unwrap(), next_type.unwrap())
        }
    }
}

fn family_change_riskyness(previous: &ColumnTypeFamily, next: &ColumnTypeFamily) -> Option<ColumnTypeChange> {
    match (previous, next) {
        (previous, next) if previous == next => None,
        (_, ColumnTypeFamily::String) => Some(ColumnTypeChange::SafeCast),
        (ColumnTypeFamily::String, ColumnTypeFamily::Int)
        | (ColumnTypeFamily::DateTime, ColumnTypeFamily::Float)
        | (ColumnTypeFamily::String, ColumnTypeFamily::Float) => Some(ColumnTypeChange::NotCastable),
        (_, _) => Some(ColumnTypeChange::RiskyCast),
    }
}

fn native_type_change_riskyness(previous: PostgresType, next: PostgresType) -> Option<ColumnTypeChange> {
    use ColumnTypeChange::*;

    let cast = || match previous {
        PostgresType::SmallInt => match next {
            PostgresType::Integer => SafeCast,
            PostgresType::BigInt => SafeCast,
            PostgresType::Decimal(params) | PostgresType::Numeric(params) => match params {
                // SmallInt can be at most three digits, so this might fail.
                Some((p, s)) if p - s < 3 => RiskyCast,
                _ => SafeCast,
            },
            PostgresType::Real => SafeCast,
            PostgresType::DoublePrecision => SafeCast,
            PostgresType::VarChar(param) | PostgresType::Char(param) => match param {
                // We can have five digits and an optional sign.
                Some(len) if len < 6 => RiskyCast,
                None => RiskyCast,
                _ => SafeCast,
            },
            PostgresType::Text => SafeCast,
            _ => NotCastable,
        },
        PostgresType::Integer => match next {
            PostgresType::SmallInt => RiskyCast,
            PostgresType::BigInt => SafeCast,
            PostgresType::Decimal(params) | PostgresType::Numeric(params) => match params {
                // Integer can be at most five digits, so this might fail.
                Some((p, s)) if p - s < 5 => RiskyCast,
                _ => SafeCast,
            },
            PostgresType::Real => SafeCast,
            PostgresType::DoublePrecision => SafeCast,
            PostgresType::VarChar(param) | PostgresType::Char(param) => match param {
                // We can have five digits and an optional sign.
                Some(len) if len < 6 => RiskyCast,
                None => RiskyCast,
                _ => SafeCast,
            },
            PostgresType::Text => SafeCast,
            _ => NotCastable,
        },
        PostgresType::BigInt => match next {
            PostgresType::SmallInt => RiskyCast,
            PostgresType::Integer => RiskyCast,
            PostgresType::Decimal(params) | PostgresType::Numeric(params) => match params {
                // Integer can be at most nineteen digits, so this might fail.
                Some((p, s)) if p - s < 19 => RiskyCast,
                _ => SafeCast,
            },
            PostgresType::Real => SafeCast,
            PostgresType::DoublePrecision => SafeCast,
            PostgresType::VarChar(param) | PostgresType::Char(param) => match param {
                // We can have twenty digits and an optional sign.
                Some(len) if len < 20 => RiskyCast,
                None => RiskyCast,
                _ => SafeCast,
            },
            PostgresType::Text => SafeCast,
            _ => NotCastable,
        },
        PostgresType::Decimal(old_params) | PostgresType::Numeric(old_params) => match next {
            PostgresType::SmallInt => match old_params {
                None => RiskyCast,
                Some((p, s)) if s > 0 => RiskyCast,
                Some((p, 0)) if p > 5 => RiskyCast,
                _ => SafeCast,
            },
            PostgresType::Integer => match old_params {
                None => RiskyCast,
                Some((p, s)) if s > 0 => RiskyCast,
                Some((p, 0)) if p > 10 => RiskyCast,
                _ => SafeCast,
            },
            PostgresType::BigInt => match old_params {
                None => RiskyCast,
                Some((p, s)) if s > 0 => RiskyCast,
                Some((p, 0)) if p > 19 => RiskyCast,
                _ => SafeCast,
            },
            PostgresType::Decimal(new_params) | PostgresType::Numeric(new_params) => match (old_params, new_params) {
                (Some((p_old, s_old)), None) => SafeCast,
                (None, Some((p_new, s_new))) if p_new < 131072 || s_new < 16383 => RiskyCast,
                // Sigh... So, numeric(4,0) to numeric(4,2) would be risky,
                // so would numeric(4,2) to numeric(4,0).
                (Some((p_old, s_old)), Some((p_new, s_new))) if p_old - s_old > p_new - s_new || s_old > s_new => {
                    RiskyCast
                }
                _ => SafeCast,
            },
            PostgresType::Real => RiskyCast,            //todo depends on params
            PostgresType::DoublePrecision => RiskyCast, //todo depends on params
            PostgresType::VarChar(length) | PostgresType::Char(length) => match (length, old_params) {
                // We must fit p digits and a possible sign to our
                // string, otherwise might truncate.
                (Some(len), Some((p, 0))) if p + 1 > len => RiskyCast,
                // We must fit p digits, a possible sign and a comma to
                // our string, otherwise might truncate.
                (Some(len), Some((p, n))) if n > 0 && p + 2 > len.into() => RiskyCast,
                //up to 131072 digits before the decimal point; up to 16383 digits after the decimal point
                (Some(len), None) if len < 131073 => RiskyCast,
                (None, _) => RiskyCast,
                _ => SafeCast,
            },
            PostgresType::Text => SafeCast,
            _ => NotCastable,
        },
        PostgresType::Real => match next {
            PostgresType::SmallInt => RiskyCast,
            PostgresType::Integer => RiskyCast,
            PostgresType::BigInt => RiskyCast,
            PostgresType::Decimal(_) | PostgresType::Numeric(_) => RiskyCast, //todo depends on params?
            PostgresType::Real => SafeCast,
            PostgresType::DoublePrecision => SafeCast,
            PostgresType::VarChar(_) | PostgresType::Char(_) => RiskyCast, //todo depends on length
            PostgresType::Text => SafeCast,
            //todo ??
            PostgresType::ByteA => SafeCast,
            PostgresType::Timestamp(_) => SafeCast,
            PostgresType::Timestamptz(_) => SafeCast,
            PostgresType::Date => SafeCast,
            PostgresType::Time(_) => SafeCast,
            PostgresType::Timetz(_) => SafeCast,
            PostgresType::Boolean => SafeCast,
            PostgresType::Bit(_) => SafeCast,
            PostgresType::VarBit(_) => SafeCast,
            PostgresType::UUID => SafeCast,
            PostgresType::Xml => SafeCast,
            PostgresType::JSON => SafeCast,
            PostgresType::JSONB => SafeCast,
        },
        //todo
        PostgresType::DoublePrecision => match next {
            PostgresType::SmallInt => SafeCast,
            PostgresType::Integer => SafeCast,
            PostgresType::BigInt => SafeCast,
            PostgresType::Decimal(_) => SafeCast,
            PostgresType::Numeric(_) => SafeCast,
            PostgresType::Real => SafeCast,
            PostgresType::DoublePrecision => SafeCast,
            PostgresType::VarChar(_) => SafeCast,
            PostgresType::Char(_) => SafeCast,
            PostgresType::Text => SafeCast,
            PostgresType::ByteA => SafeCast,
            PostgresType::Timestamp(_) => SafeCast,
            PostgresType::Timestamptz(_) => SafeCast,
            PostgresType::Date => SafeCast,
            PostgresType::Time(_) => SafeCast,
            PostgresType::Timetz(_) => SafeCast,
            PostgresType::Boolean => SafeCast,
            PostgresType::Bit(_) => SafeCast,
            PostgresType::VarBit(_) => SafeCast,
            PostgresType::UUID => SafeCast,
            PostgresType::Xml => SafeCast,
            PostgresType::JSON => SafeCast,
            PostgresType::JSONB => SafeCast,
        },
        //todo later
        PostgresType::VarChar(_) => match next {
            PostgresType::SmallInt => SafeCast,
            PostgresType::Integer => SafeCast,
            PostgresType::BigInt => SafeCast,
            PostgresType::Decimal(_) => SafeCast,
            PostgresType::Numeric(_) => SafeCast,
            PostgresType::Real => SafeCast,
            PostgresType::DoublePrecision => SafeCast,
            PostgresType::VarChar(_) => SafeCast,
            PostgresType::Char(_) => SafeCast,
            PostgresType::Text => SafeCast,
            PostgresType::ByteA => SafeCast,
            PostgresType::Timestamp(_) => SafeCast,
            PostgresType::Timestamptz(_) => SafeCast,
            PostgresType::Date => SafeCast,
            PostgresType::Time(_) => SafeCast,
            PostgresType::Timetz(_) => SafeCast,
            PostgresType::Boolean => SafeCast,
            PostgresType::Bit(_) => SafeCast,
            PostgresType::VarBit(_) => SafeCast,
            PostgresType::UUID => SafeCast,
            PostgresType::Xml => SafeCast,
            PostgresType::JSON => SafeCast,
            PostgresType::JSONB => SafeCast,
        },
        PostgresType::Char(_) => match next {
            PostgresType::SmallInt => SafeCast,
            PostgresType::Integer => SafeCast,
            PostgresType::BigInt => SafeCast,
            PostgresType::Decimal(_) => SafeCast,
            PostgresType::Numeric(_) => SafeCast,
            PostgresType::Real => SafeCast,
            PostgresType::DoublePrecision => SafeCast,
            PostgresType::VarChar(_) => SafeCast,
            PostgresType::Char(_) => SafeCast,
            PostgresType::Text => SafeCast,
            PostgresType::ByteA => SafeCast,
            PostgresType::Timestamp(_) => SafeCast,
            PostgresType::Timestamptz(_) => SafeCast,
            PostgresType::Date => SafeCast,
            PostgresType::Time(_) => SafeCast,
            PostgresType::Timetz(_) => SafeCast,
            PostgresType::Boolean => SafeCast,
            PostgresType::Bit(_) => SafeCast,
            PostgresType::VarBit(_) => SafeCast,
            PostgresType::UUID => SafeCast,
            PostgresType::Xml => SafeCast,
            PostgresType::JSON => SafeCast,
            PostgresType::JSONB => SafeCast,
        },
        PostgresType::Text => match next {
            PostgresType::SmallInt => SafeCast,
            PostgresType::Integer => SafeCast,
            PostgresType::BigInt => SafeCast,
            PostgresType::Decimal(_) => SafeCast,
            PostgresType::Numeric(_) => SafeCast,
            PostgresType::Real => SafeCast,
            PostgresType::DoublePrecision => SafeCast,
            PostgresType::VarChar(_) => SafeCast,
            PostgresType::Char(_) => SafeCast,
            PostgresType::Text => SafeCast,
            PostgresType::ByteA => SafeCast,
            PostgresType::Timestamp(_) => SafeCast,
            PostgresType::Timestamptz(_) => SafeCast,
            PostgresType::Date => SafeCast,
            PostgresType::Time(_) => SafeCast,
            PostgresType::Timetz(_) => SafeCast,
            PostgresType::Boolean => SafeCast,
            PostgresType::Bit(_) => SafeCast,
            PostgresType::VarBit(_) => SafeCast,
            PostgresType::UUID => SafeCast,
            PostgresType::Xml => SafeCast,
            PostgresType::JSON => SafeCast,
            PostgresType::JSONB => SafeCast,
        },
        PostgresType::ByteA => match next {
            PostgresType::SmallInt => SafeCast,
            PostgresType::Integer => SafeCast,
            PostgresType::BigInt => SafeCast,
            PostgresType::Decimal(_) => SafeCast,
            PostgresType::Numeric(_) => SafeCast,
            PostgresType::Real => SafeCast,
            PostgresType::DoublePrecision => SafeCast,
            PostgresType::VarChar(_) => SafeCast,
            PostgresType::Char(_) => SafeCast,
            PostgresType::Text => SafeCast,
            PostgresType::ByteA => SafeCast,
            PostgresType::Timestamp(_) => SafeCast,
            PostgresType::Timestamptz(_) => SafeCast,
            PostgresType::Date => SafeCast,
            PostgresType::Time(_) => SafeCast,
            PostgresType::Timetz(_) => SafeCast,
            PostgresType::Boolean => SafeCast,
            PostgresType::Bit(_) => SafeCast,
            PostgresType::VarBit(_) => SafeCast,
            PostgresType::UUID => SafeCast,
            PostgresType::Xml => SafeCast,
            PostgresType::JSON => SafeCast,
            PostgresType::JSONB => SafeCast,
        },
        PostgresType::Timestamp(_) => match next {
            PostgresType::SmallInt => SafeCast,
            PostgresType::Integer => SafeCast,
            PostgresType::BigInt => SafeCast,
            PostgresType::Decimal(_) => SafeCast,
            PostgresType::Numeric(_) => SafeCast,
            PostgresType::Real => SafeCast,
            PostgresType::DoublePrecision => SafeCast,
            PostgresType::VarChar(_) => SafeCast,
            PostgresType::Char(_) => SafeCast,
            PostgresType::Text => SafeCast,
            PostgresType::ByteA => SafeCast,
            PostgresType::Timestamp(_) => SafeCast,
            PostgresType::Timestamptz(_) => SafeCast,
            PostgresType::Date => SafeCast,
            PostgresType::Time(_) => SafeCast,
            PostgresType::Timetz(_) => SafeCast,
            PostgresType::Boolean => SafeCast,
            PostgresType::Bit(_) => SafeCast,
            PostgresType::VarBit(_) => SafeCast,
            PostgresType::UUID => SafeCast,
            PostgresType::Xml => SafeCast,
            PostgresType::JSON => SafeCast,
            PostgresType::JSONB => SafeCast,
        },
        PostgresType::Timestamptz(_) => match next {
            PostgresType::SmallInt => SafeCast,
            PostgresType::Integer => SafeCast,
            PostgresType::BigInt => SafeCast,
            PostgresType::Decimal(_) => SafeCast,
            PostgresType::Numeric(_) => SafeCast,
            PostgresType::Real => SafeCast,
            PostgresType::DoublePrecision => SafeCast,
            PostgresType::VarChar(_) => SafeCast,
            PostgresType::Char(_) => SafeCast,
            PostgresType::Text => SafeCast,
            PostgresType::ByteA => SafeCast,
            PostgresType::Timestamp(_) => SafeCast,
            PostgresType::Timestamptz(_) => SafeCast,
            PostgresType::Date => SafeCast,
            PostgresType::Time(_) => SafeCast,
            PostgresType::Timetz(_) => SafeCast,
            PostgresType::Boolean => SafeCast,
            PostgresType::Bit(_) => SafeCast,
            PostgresType::VarBit(_) => SafeCast,
            PostgresType::UUID => SafeCast,
            PostgresType::Xml => SafeCast,
            PostgresType::JSON => SafeCast,
            PostgresType::JSONB => SafeCast,
        },
        PostgresType::Date => match next {
            PostgresType::SmallInt => SafeCast,
            PostgresType::Integer => SafeCast,
            PostgresType::BigInt => SafeCast,
            PostgresType::Decimal(_) => SafeCast,
            PostgresType::Numeric(_) => SafeCast,
            PostgresType::Real => SafeCast,
            PostgresType::DoublePrecision => SafeCast,
            PostgresType::VarChar(_) => SafeCast,
            PostgresType::Char(_) => SafeCast,
            PostgresType::Text => SafeCast,
            PostgresType::ByteA => SafeCast,
            PostgresType::Timestamp(_) => SafeCast,
            PostgresType::Timestamptz(_) => SafeCast,
            PostgresType::Date => SafeCast,
            PostgresType::Time(_) => SafeCast,
            PostgresType::Timetz(_) => SafeCast,
            PostgresType::Boolean => SafeCast,
            PostgresType::Bit(_) => SafeCast,
            PostgresType::VarBit(_) => SafeCast,
            PostgresType::UUID => SafeCast,
            PostgresType::Xml => SafeCast,
            PostgresType::JSON => SafeCast,
            PostgresType::JSONB => SafeCast,
        },
        PostgresType::Time(_) => match next {
            PostgresType::SmallInt => SafeCast,
            PostgresType::Integer => SafeCast,
            PostgresType::BigInt => SafeCast,
            PostgresType::Decimal(_) => SafeCast,
            PostgresType::Numeric(_) => SafeCast,
            PostgresType::Real => SafeCast,
            PostgresType::DoublePrecision => SafeCast,
            PostgresType::VarChar(_) => SafeCast,
            PostgresType::Char(_) => SafeCast,
            PostgresType::Text => SafeCast,
            PostgresType::ByteA => SafeCast,
            PostgresType::Timestamp(_) => SafeCast,
            PostgresType::Timestamptz(_) => SafeCast,
            PostgresType::Date => SafeCast,
            PostgresType::Time(_) => SafeCast,
            PostgresType::Timetz(_) => SafeCast,
            PostgresType::Boolean => SafeCast,
            PostgresType::Bit(_) => SafeCast,
            PostgresType::VarBit(_) => SafeCast,
            PostgresType::UUID => SafeCast,
            PostgresType::Xml => SafeCast,
            PostgresType::JSON => SafeCast,
            PostgresType::JSONB => SafeCast,
        },
        PostgresType::Timetz(_) => match next {
            PostgresType::SmallInt => SafeCast,
            PostgresType::Integer => SafeCast,
            PostgresType::BigInt => SafeCast,
            PostgresType::Decimal(_) => SafeCast,
            PostgresType::Numeric(_) => SafeCast,
            PostgresType::Real => SafeCast,
            PostgresType::DoublePrecision => SafeCast,
            PostgresType::VarChar(_) => SafeCast,
            PostgresType::Char(_) => SafeCast,
            PostgresType::Text => SafeCast,
            PostgresType::ByteA => SafeCast,
            PostgresType::Timestamp(_) => SafeCast,
            PostgresType::Timestamptz(_) => SafeCast,
            PostgresType::Date => SafeCast,
            PostgresType::Time(_) => SafeCast,
            PostgresType::Timetz(_) => SafeCast,
            PostgresType::Boolean => SafeCast,
            PostgresType::Bit(_) => SafeCast,
            PostgresType::VarBit(_) => SafeCast,
            PostgresType::UUID => SafeCast,
            PostgresType::Xml => SafeCast,
            PostgresType::JSON => SafeCast,
            PostgresType::JSONB => SafeCast,
        },
        PostgresType::Boolean => match next {
            PostgresType::SmallInt => SafeCast,
            PostgresType::Integer => SafeCast,
            PostgresType::BigInt => SafeCast,
            PostgresType::Decimal(_) => SafeCast,
            PostgresType::Numeric(_) => SafeCast,
            PostgresType::Real => SafeCast,
            PostgresType::DoublePrecision => SafeCast,
            PostgresType::VarChar(_) => SafeCast,
            PostgresType::Char(_) => SafeCast,
            PostgresType::Text => SafeCast,
            PostgresType::ByteA => SafeCast,
            PostgresType::Timestamp(_) => SafeCast,
            PostgresType::Timestamptz(_) => SafeCast,
            PostgresType::Date => SafeCast,
            PostgresType::Time(_) => SafeCast,
            PostgresType::Timetz(_) => SafeCast,
            PostgresType::Boolean => SafeCast,
            PostgresType::Bit(_) => SafeCast,
            PostgresType::VarBit(_) => SafeCast,
            PostgresType::UUID => SafeCast,
            PostgresType::Xml => SafeCast,
            PostgresType::JSON => SafeCast,
            PostgresType::JSONB => SafeCast,
        },
        PostgresType::Bit(_) => match next {
            PostgresType::SmallInt => SafeCast,
            PostgresType::Integer => SafeCast,
            PostgresType::BigInt => SafeCast,
            PostgresType::Decimal(_) => SafeCast,
            PostgresType::Numeric(_) => SafeCast,
            PostgresType::Real => SafeCast,
            PostgresType::DoublePrecision => SafeCast,
            PostgresType::VarChar(_) => SafeCast,
            PostgresType::Char(_) => SafeCast,
            PostgresType::Text => SafeCast,
            PostgresType::ByteA => SafeCast,
            PostgresType::Timestamp(_) => SafeCast,
            PostgresType::Timestamptz(_) => SafeCast,
            PostgresType::Date => SafeCast,
            PostgresType::Time(_) => SafeCast,
            PostgresType::Timetz(_) => SafeCast,
            PostgresType::Boolean => SafeCast,
            PostgresType::Bit(_) => SafeCast,
            PostgresType::VarBit(_) => SafeCast,
            PostgresType::UUID => SafeCast,
            PostgresType::Xml => SafeCast,
            PostgresType::JSON => SafeCast,
            PostgresType::JSONB => SafeCast,
        },
        PostgresType::VarBit(_) => match next {
            PostgresType::SmallInt => SafeCast,
            PostgresType::Integer => SafeCast,
            PostgresType::BigInt => SafeCast,
            PostgresType::Decimal(_) => SafeCast,
            PostgresType::Numeric(_) => SafeCast,
            PostgresType::Real => SafeCast,
            PostgresType::DoublePrecision => SafeCast,
            PostgresType::VarChar(_) => SafeCast,
            PostgresType::Char(_) => SafeCast,
            PostgresType::Text => SafeCast,
            PostgresType::ByteA => SafeCast,
            PostgresType::Timestamp(_) => SafeCast,
            PostgresType::Timestamptz(_) => SafeCast,
            PostgresType::Date => SafeCast,
            PostgresType::Time(_) => SafeCast,
            PostgresType::Timetz(_) => SafeCast,
            PostgresType::Boolean => SafeCast,
            PostgresType::Bit(_) => SafeCast,
            PostgresType::VarBit(_) => SafeCast,
            PostgresType::UUID => SafeCast,
            PostgresType::Xml => SafeCast,
            PostgresType::JSON => SafeCast,
            PostgresType::JSONB => SafeCast,
        },
        PostgresType::UUID => match next {
            PostgresType::SmallInt => SafeCast,
            PostgresType::Integer => SafeCast,
            PostgresType::BigInt => SafeCast,
            PostgresType::Decimal(_) => SafeCast,
            PostgresType::Numeric(_) => SafeCast,
            PostgresType::Real => SafeCast,
            PostgresType::DoublePrecision => SafeCast,
            PostgresType::VarChar(_) => SafeCast,
            PostgresType::Char(_) => SafeCast,
            PostgresType::Text => SafeCast,
            PostgresType::ByteA => SafeCast,
            PostgresType::Timestamp(_) => SafeCast,
            PostgresType::Timestamptz(_) => SafeCast,
            PostgresType::Date => SafeCast,
            PostgresType::Time(_) => SafeCast,
            PostgresType::Timetz(_) => SafeCast,
            PostgresType::Boolean => SafeCast,
            PostgresType::Bit(_) => SafeCast,
            PostgresType::VarBit(_) => SafeCast,
            PostgresType::UUID => SafeCast,
            PostgresType::Xml => SafeCast,
            PostgresType::JSON => SafeCast,
            PostgresType::JSONB => SafeCast,
        },
        PostgresType::Xml => match next {
            PostgresType::SmallInt => SafeCast,
            PostgresType::Integer => SafeCast,
            PostgresType::BigInt => SafeCast,
            PostgresType::Decimal(_) => SafeCast,
            PostgresType::Numeric(_) => SafeCast,
            PostgresType::Real => SafeCast,
            PostgresType::DoublePrecision => SafeCast,
            PostgresType::VarChar(_) => SafeCast,
            PostgresType::Char(_) => SafeCast,
            PostgresType::Text => SafeCast,
            PostgresType::ByteA => SafeCast,
            PostgresType::Timestamp(_) => SafeCast,
            PostgresType::Timestamptz(_) => SafeCast,
            PostgresType::Date => SafeCast,
            PostgresType::Time(_) => SafeCast,
            PostgresType::Timetz(_) => SafeCast,
            PostgresType::Boolean => SafeCast,
            PostgresType::Bit(_) => SafeCast,
            PostgresType::VarBit(_) => SafeCast,
            PostgresType::UUID => SafeCast,
            PostgresType::Xml => SafeCast,
            PostgresType::JSON => SafeCast,
            PostgresType::JSONB => SafeCast,
        },
        PostgresType::JSON => match next {
            PostgresType::SmallInt => SafeCast,
            PostgresType::Integer => SafeCast,
            PostgresType::BigInt => SafeCast,
            PostgresType::Decimal(_) => SafeCast,
            PostgresType::Numeric(_) => SafeCast,
            PostgresType::Real => SafeCast,
            PostgresType::DoublePrecision => SafeCast,
            PostgresType::VarChar(_) => SafeCast,
            PostgresType::Char(_) => SafeCast,
            PostgresType::Text => SafeCast,
            PostgresType::ByteA => SafeCast,
            PostgresType::Timestamp(_) => SafeCast,
            PostgresType::Timestamptz(_) => SafeCast,
            PostgresType::Date => SafeCast,
            PostgresType::Time(_) => SafeCast,
            PostgresType::Timetz(_) => SafeCast,
            PostgresType::Boolean => SafeCast,
            PostgresType::Bit(_) => SafeCast,
            PostgresType::VarBit(_) => SafeCast,
            PostgresType::UUID => SafeCast,
            PostgresType::Xml => SafeCast,
            PostgresType::JSON => SafeCast,
            PostgresType::JSONB => SafeCast,
        },
        PostgresType::JSONB => match next {
            PostgresType::SmallInt => SafeCast,
            PostgresType::Integer => SafeCast,
            PostgresType::BigInt => SafeCast,
            PostgresType::Decimal(_) => SafeCast,
            PostgresType::Numeric(_) => SafeCast,
            PostgresType::Real => SafeCast,
            PostgresType::DoublePrecision => SafeCast,
            PostgresType::VarChar(_) => SafeCast,
            PostgresType::Char(_) => SafeCast,
            PostgresType::Text => SafeCast,
            PostgresType::ByteA => SafeCast,
            PostgresType::Timestamp(_) => SafeCast,
            PostgresType::Timestamptz(_) => SafeCast,
            PostgresType::Date => SafeCast,
            PostgresType::Time(_) => SafeCast,
            PostgresType::Timetz(_) => SafeCast,
            PostgresType::Boolean => SafeCast,
            PostgresType::Bit(_) => SafeCast,
            PostgresType::VarBit(_) => SafeCast,
            PostgresType::UUID => SafeCast,
            PostgresType::Xml => SafeCast,
            PostgresType::JSON => SafeCast,
            PostgresType::JSONB => SafeCast,
        },
    };

    if previous == next {
        None
    } else {
        Some(cast())
    }
}

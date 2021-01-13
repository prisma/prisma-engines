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
        use ColumnTypeChange::*;
        let native_types_enabled = self.features().contains(MigrationFeature::NativeTypes);
        let from_list_to_scalar = differ.previous.arity().is_list() && !differ.next.arity().is_list();
        let from_scalar_to_list = !differ.previous.arity().is_list() && differ.next.arity().is_list();

        if !native_types_enabled {
            let previous_family = differ.previous.column_type_family();
            let next_family = differ.next.column_type_family();
            match (previous_family, next_family) {
                (_, ColumnTypeFamily::String) if from_list_to_scalar => Some(SafeCast),
                (_, _) if from_list_to_scalar => Some(NotCastable),
                (ColumnTypeFamily::Decimal, ColumnTypeFamily::Decimal)
                | (ColumnTypeFamily::Float, ColumnTypeFamily::Float)
                | (ColumnTypeFamily::Decimal, ColumnTypeFamily::Float)
                | (ColumnTypeFamily::Float, ColumnTypeFamily::Decimal)
                | (ColumnTypeFamily::Binary, ColumnTypeFamily::Binary)
                    if from_scalar_to_list =>
                {
                    Some(NotCastable)
                }
                (previous, next) => family_change_riskyness(previous, next),
            }
        } else {
            let previous_type: Option<PostgresType> = differ.previous.column_native_type();
            let next_type: Option<PostgresType> = differ.next.column_native_type();

            match (previous_type, next_type) {
                (_, Some(PostgresType::Text)) if from_list_to_scalar => Some(SafeCast),
                (_, Some(PostgresType::VarChar(None))) if from_list_to_scalar => Some(SafeCast),
                (_, Some(PostgresType::VarChar(_))) if from_list_to_scalar => Some(RiskyCast),
                (_, Some(PostgresType::Char(_))) if from_list_to_scalar => Some(RiskyCast),
                (_, _) if from_scalar_to_list => Some(NotCastable),
                (Some(previous), Some(next)) => native_type_change_riskyness(previous, next),
                // Unsupported types will have None as Native type
                _ => Some(NotCastable),
            }
        }
    }
}

fn family_change_riskyness(previous: &ColumnTypeFamily, next: &ColumnTypeFamily) -> Option<ColumnTypeChange> {
    // todo this is not completely correct yet it seems
    match (previous, next) {
        (previous, next) if previous == next => None,
        (_, ColumnTypeFamily::String) => Some(ColumnTypeChange::SafeCast),
        (ColumnTypeFamily::String, ColumnTypeFamily::Int)
        | (ColumnTypeFamily::DateTime, ColumnTypeFamily::Float)
        | (ColumnTypeFamily::String, ColumnTypeFamily::Float)
        | (ColumnTypeFamily::Decimal, ColumnTypeFamily::DateTime) => Some(ColumnTypeChange::NotCastable),
        (_, _) => Some(ColumnTypeChange::RiskyCast),
    }
}

fn native_type_change_riskyness(previous: PostgresType, next: PostgresType) -> Option<ColumnTypeChange> {
    use native_types::PostgresType::*;
    use ColumnTypeChange::*;

    // varchar / varbit without param=> unlimited length
    // char / bit without param => length is 1
    let next_is_char = || matches!(next, Char(_));

    let cast = || match previous {
        SmallInt => match next {
            Integer => SafeCast,
            BigInt => SafeCast,
            Decimal(params) | Numeric(params) => match params {
                // SmallInt can be at most three digits, so this might fail.
                Some((p, s)) if p - s < 3 => RiskyCast,
                _ => SafeCast,
            },
            Real => SafeCast,
            DoublePrecision => SafeCast,
            VarChar(param) | Char(param) => match param {
                // Smallint can have three digits and an optional sign.
                Some(len) if len < 4 => RiskyCast,
                None if next_is_char() => RiskyCast,
                _ => SafeCast,
            },
            Text => SafeCast,
            _ => NotCastable,
        },
        Integer => match next {
            SmallInt => RiskyCast,
            BigInt => SafeCast,
            Decimal(params) | Numeric(params) => match params {
                // Integer can be at most 10 digits, so this might fail.
                Some((p, s)) if p - s < 10 => RiskyCast,
                _ => SafeCast,
            },
            Real => SafeCast,
            DoublePrecision => SafeCast,
            VarChar(param) | Char(param) => match param {
                // Integer can have five digits and an optional sign.
                Some(len) if len < 11 => RiskyCast,
                None if next_is_char() => RiskyCast,
                _ => SafeCast,
            },
            Text => SafeCast,
            _ => NotCastable,
        },
        BigInt => match next {
            SmallInt => RiskyCast,
            Integer => RiskyCast,
            Decimal(params) | Numeric(params) => match params {
                // Bigint can be at most nineteen digits, so this might fail.
                Some((p, s)) if p - s < 19 => RiskyCast,
                _ => SafeCast,
            },
            Real => SafeCast,
            DoublePrecision => SafeCast,
            VarChar(param) | Char(param) => match param {
                // Bigint can have twenty digits and an optional sign.
                Some(len) if len < 20 => RiskyCast,
                None if next_is_char() => RiskyCast,
                _ => SafeCast,
            },
            Text => SafeCast,
            _ => NotCastable,
        },
        Decimal(old_params) | Numeric(old_params) => match next {
            SmallInt => match old_params {
                None => RiskyCast,
                Some((_, s)) if s > 0 => RiskyCast,
                Some((p, 0)) if p > 2 => RiskyCast,
                _ => SafeCast,
            },
            Integer => match old_params {
                None => RiskyCast,
                Some((_, s)) if s > 0 => RiskyCast,
                Some((p, 0)) if p > 9 => RiskyCast,
                _ => SafeCast,
            },
            BigInt => match old_params {
                None => RiskyCast,
                Some((_, s)) if s > 0 => RiskyCast,
                Some((p, 0)) if p > 18 => RiskyCast,
                _ => SafeCast,
            },
            Decimal(new_params) | Numeric(new_params) => match (old_params, new_params) {
                (Some(_), None) => SafeCast,
                (None, Some((p_new, s_new))) if p_new < 131072 || s_new < 16383 => RiskyCast,
                // Sigh... So, numeric(4,0) to numeric(4,2) would be risky,
                // so would numeric(4,2) to numeric(4,0).
                (Some((p_old, s_old)), Some((p_new, s_new))) if p_old - s_old > p_new - s_new || s_old > s_new => {
                    RiskyCast
                }
                _ => SafeCast,
            },
            //todo this is the same as in mssql, but could be more finegrained
            Real => RiskyCast,
            DoublePrecision => RiskyCast,
            VarChar(length) | Char(length) => match (length, old_params) {
                // We must fit p digits and a possible sign to our
                // string, otherwise might truncate.
                (Some(len), Some((p, 0))) if p + 1 > len => RiskyCast,
                // We must fit p digits, a possible sign and a comma to
                // our string, otherwise might truncate.
                (Some(len), Some((p, n))) if n > 0 && p + 2 > len.into() => RiskyCast,
                //up to 131072 digits before the decimal point; up to 16383 digits after the decimal point
                (Some(len), None) if len < 131073 => RiskyCast,
                (None, _) if next_is_char() => RiskyCast,
                (None, _) => SafeCast,
                _ => SafeCast,
            },
            Text => SafeCast,
            _ => NotCastable,
        },
        Real => match next {
            SmallInt => RiskyCast,
            Integer => RiskyCast,
            BigInt => RiskyCast,
            Decimal(_) | Numeric(_) => RiskyCast,
            Real => SafeCast,
            DoublePrecision => SafeCast,
            VarChar(len) | Char(len) => match len {
                // If float, we can have 47 characters including the sign and comma.
                Some(len) if len < 47 => RiskyCast,
                None if next_is_char() => RiskyCast,
                _ => SafeCast,
            },
            Text => SafeCast,
            _ => NotCastable,
        },
        DoublePrecision => match next {
            SmallInt => RiskyCast,
            Integer => RiskyCast,
            BigInt => RiskyCast,
            Decimal(_) | Numeric(_) => RiskyCast,
            Real => RiskyCast,
            DoublePrecision => SafeCast,
            VarChar(len) | Char(len) => match len {
                // If double, we can have 317 characters including the sign and comma.
                Some(len) if len < 317 => RiskyCast,
                None if next_is_char() => RiskyCast,
                _ => SafeCast,
            },
            Text => SafeCast,
            _ => NotCastable,
        },
        VarChar(old_param) => match next {
            Text => SafeCast,
            VarChar(new_param) | Char(new_param) => match (old_param, new_param) {
                (None, None) if next_is_char() => RiskyCast,
                (None, Some(_)) => RiskyCast,
                (Some(1), None) => SafeCast,
                (Some(_), None) if next_is_char() => RiskyCast,
                (Some(_), None) => SafeCast,
                (Some(old_length), Some(new_length)) if old_length > new_length => RiskyCast,
                _ => SafeCast,
            },
            _ => NotCastable,
        },
        Char(old_param) => match next {
            Text => SafeCast,
            VarChar(new_param) | Char(new_param) => match (old_param, new_param) {
                (None, _) => SafeCast,
                (Some(1), None) => SafeCast,
                (Some(_), None) if next_is_char() => RiskyCast,
                (Some(_), None) => SafeCast,
                (Some(old_length), Some(new_length)) if old_length > new_length => RiskyCast,
                _ => SafeCast,
            },
            _ => NotCastable,
        },
        Text => match next {
            Text | VarChar(None) => SafeCast,
            VarChar(_) | Char(_) => RiskyCast,
            _ => NotCastable,
        },
        ByteA => match next {
            Text | VarChar(None) => SafeCast,
            VarChar(Some(length)) | Char(Some(length)) if length > 2 => RiskyCast,
            _ => NotCastable,
        },
        Timestamp(_) => match next {
            Text | VarChar(None) => SafeCast,
            Char(Some(len)) | VarChar(Some(len)) if len > 22 => SafeCast,
            Timestamp(_) | Timestamptz(_) | Date | Time(_) | Timetz(_) => SafeCast,
            _ => NotCastable,
        },
        Timestamptz(_) => match next {
            Text | VarChar(None) => SafeCast,
            Char(Some(len)) | VarChar(Some(len)) if len > 27 => SafeCast,
            Timestamp(_) | Timestamptz(_) | Date | Time(_) | Timetz(_) => SafeCast,
            _ => NotCastable,
        },
        Date => match next {
            Text | VarChar(None) => SafeCast,
            Char(Some(len)) | VarChar(Some(len)) if len > 27 => SafeCast,
            Timestamp(_) | Timestamptz(_) => SafeCast,
            _ => NotCastable,
        },
        Time(_) => match next {
            Text | VarChar(None) => SafeCast,
            Char(Some(len)) | VarChar(Some(len)) if len > 13 => SafeCast,
            Timetz(_) => SafeCast,
            _ => NotCastable,
        },
        Timetz(_) => match next {
            Text | VarChar(None) => SafeCast,
            Char(Some(len)) | VarChar(Some(len)) if len > 18 => SafeCast,
            Timetz(_) | Time(_) => SafeCast,
            _ => NotCastable,
        },
        Boolean => match next {
            Text | VarChar(_) => SafeCast,
            Char(Some(length)) if length > 4 => SafeCast,
            Char(Some(length)) if length > 3 => RiskyCast,
            _ => NotCastable,
        },
        Bit(None) => match next {
            Text | VarChar(_) | Char(_) | VarBit(_) => SafeCast,
            _ => NotCastable,
        },
        Bit(Some(length)) => match next {
            Text | VarChar(None) | VarBit(None) => SafeCast,
            VarChar(Some(new_length)) if new_length >= length => SafeCast,
            VarBit(Some(new_length)) | Char(Some(new_length)) if new_length >= length => SafeCast,
            _ => NotCastable,
        },

        VarBit(None) => match next {
            Text | VarChar(None) => SafeCast,
            VarChar(_) | Char(_) | Bit(_) => RiskyCast,
            _ => NotCastable,
        },
        VarBit(Some(length)) => match next {
            Text | VarChar(None) | VarBit(None) => SafeCast,
            VarBit(Some(new_length)) if new_length > length => SafeCast,
            VarChar(Some(new_length)) | Char(Some(new_length)) if new_length >= length => SafeCast,
            Bit(Some(new_length)) if new_length <= length => RiskyCast,
            Bit(None) => RiskyCast,
            Char(_) | VarChar(_) => RiskyCast,
            _ => NotCastable,
        },
        UUID => match next {
            Text | VarChar(None) => SafeCast,
            VarChar(Some(length)) | Char(Some(length)) if length > 31 => SafeCast,
            _ => NotCastable,
        },
        Xml => match next {
            Text | VarChar(None) => SafeCast,
            VarChar(_) | Char(_) => RiskyCast,
            _ => NotCastable,
        },
        JSON => match next {
            Text | JSONB | VarChar(None) => SafeCast,
            VarChar(_) | Char(_) => RiskyCast,
            _ => NotCastable,
        },
        JSONB => match next {
            Text | JSON | VarChar(None) => SafeCast,
            VarChar(_) | Char(_) => RiskyCast,
            _ => NotCastable,
        },
    };

    if previous == next {
        None
    } else {
        Some(cast())
    }
}

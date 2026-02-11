use crate::{
    migration_pair::MigrationPair,
    sql_migration::SqlMigrationStep,
    sql_schema_differ::{
        ColumnChanges, SqlSchemaDifferFlavour, column::ColumnTypeChange, differ_database::DifferDatabase,
        table::TableDiffer,
    },
};
use psl::builtin_connectors::{MsSqlType, MsSqlTypeParameter};
use sql_schema_describer::{self as sql, ColumnTypeFamily, TableColumnId, mssql::MssqlSchemaExt};

#[derive(Debug, Default)]
pub struct MssqlSchemaDifferFlavour;

impl SqlSchemaDifferFlavour for MssqlSchemaDifferFlavour {
    fn can_rename_foreign_key(&self) -> bool {
        true
    }

    fn indexes_match(&self, a: sql::IndexWalker<'_>, b: sql::IndexWalker<'_>) -> bool {
        let mssql_ext_previous: &MssqlSchemaExt = a.schema.downcast_connector_data();
        let mssql_ext_next: &MssqlSchemaExt = b.schema.downcast_connector_data();

        mssql_ext_previous.index_is_clustered(a.id) == mssql_ext_next.index_is_clustered(b.id)
            && a.predicate() == b.predicate()
    }

    fn should_skip_index_for_new_table(&self, index: sql::IndexWalker<'_>) -> bool {
        index.is_unique() && index.predicate().is_none()
    }

    fn should_recreate_the_primary_key_on_column_recreate(&self) -> bool {
        true
    }

    fn set_tables_to_redefine(&self, db: &mut DifferDatabase<'_>) {
        let autoincrement_changed = db
            .table_pairs()
            .filter(|differ| {
                differ
                    .column_pairs()
                    .any(|c| db.column_changes_for_walkers(c).autoincrement_changed())
            })
            .map(|t| t.table_ids());

        let all_columns_of_the_table_gets_dropped = db
            .table_pairs()
            .filter(|tables| {
                tables.column_pairs().all(|columns| {
                    let type_change = self.column_type_change(columns);
                    matches!(type_change, Some(ColumnTypeChange::NotCastable))
                })
            })
            .map(|t| t.table_ids());

        db.tables_to_redefine = autoincrement_changed
            .chain(all_columns_of_the_table_gets_dropped)
            .collect();
    }

    fn column_type_change(&self, differ: MigrationPair<sql::TableColumnWalker<'_>>) -> Option<ColumnTypeChange> {
        let previous_family = differ.previous.column_type_family();
        let next_family = differ.next.column_type_family();
        let previous_type: Option<&MsSqlType> = differ.previous.column_native_type();
        let next_type: Option<&MsSqlType> = differ.next.column_native_type();

        match (previous_type, next_type) {
            (None, _) | (_, None) => family_change_riskyness(previous_family, next_family),
            (Some(previous), Some(next)) => native_type_change_riskyness(previous, next),
        }
    }

    fn primary_key_changed(&self, tables: MigrationPair<sql::TableWalker<'_>>) -> bool {
        let pk_clusterings = tables.map(|t| {
            let ext: &MssqlSchemaExt = t.schema.downcast_connector_data();
            t.primary_key().map(|pk| ext.index_is_clustered(pk.id)).unwrap_or(false)
        });
        pk_clusterings.previous != pk_clusterings.next
    }

    fn push_index_changes_for_column_changes(
        &self,
        table: &TableDiffer<'_, '_>,
        column_id: MigrationPair<TableColumnId>,
        column_changes: ColumnChanges,
        steps: &mut Vec<SqlMigrationStep>,
    ) {
        if !column_changes.type_changed() {
            return;
        }

        for dropped_index in table.index_pairs().filter(|pair| {
            pair.previous
                .columns()
                .any(|col| col.as_column().id == column_id.previous)
        }) {
            steps.push(SqlMigrationStep::DropIndex {
                index_id: dropped_index.previous.id,
            })
        }

        for created_index in table
            .index_pairs()
            .filter(|pair| pair.next.columns().any(|col| col.as_column().id == column_id.next))
        {
            steps.push(SqlMigrationStep::CreateIndex {
                table_id: (None, table.next().id),
                index_id: created_index.next.id,
                from_drop_and_recreate: false,
            })
        }
    }
}

fn family_change_riskyness(previous: &ColumnTypeFamily, next: &ColumnTypeFamily) -> Option<ColumnTypeChange> {
    match (previous, next) {
        (prev, next) if prev == next => None,
        (_, ColumnTypeFamily::String) => Some(ColumnTypeChange::SafeCast),
        (ColumnTypeFamily::String, ColumnTypeFamily::Int)
        | (ColumnTypeFamily::DateTime, ColumnTypeFamily::Float)
        | (ColumnTypeFamily::String, ColumnTypeFamily::Float) => Some(ColumnTypeChange::NotCastable),
        (_, _) => Some(ColumnTypeChange::RiskyCast),
    }
}

fn native_type_change_riskyness(previous: &MsSqlType, next: &MsSqlType) -> Option<ColumnTypeChange> {
    use ColumnTypeChange::*;
    use MsSqlTypeParameter::*;

    let cast = || match previous {
        // Bit, as in booleans. 1 or 0.
        MsSqlType::Bit => match next {
            MsSqlType::TinyInt => SafeCast,
            MsSqlType::SmallInt => SafeCast,
            MsSqlType::Int => SafeCast,
            MsSqlType::BigInt => SafeCast,
            MsSqlType::Decimal(_) => SafeCast,
            MsSqlType::Money => SafeCast,
            MsSqlType::SmallMoney => SafeCast,
            MsSqlType::Float(_) => SafeCast,
            MsSqlType::Real => SafeCast,
            MsSqlType::DateTime => SafeCast,
            MsSqlType::SmallDateTime => SafeCast,
            MsSqlType::Binary(_) => SafeCast,
            MsSqlType::VarBinary(_) => SafeCast,
            MsSqlType::Bit => SafeCast,
            MsSqlType::Char(_) => SafeCast,
            MsSqlType::NChar(_) => SafeCast,
            MsSqlType::VarChar(_) => SafeCast,
            MsSqlType::NVarChar(_) => SafeCast,
            _ => NotCastable,
        },

        // Maps to u8.
        MsSqlType::TinyInt => match next {
            MsSqlType::TinyInt => SafeCast,
            MsSqlType::SmallInt => SafeCast,
            MsSqlType::Int => SafeCast,
            MsSqlType::BigInt => SafeCast,
            MsSqlType::Decimal(params) => match params {
                // TinyInt can be at most three digits, so this might fail.
                Some((p, s)) if p - s < 3 => RiskyCast,
                _ => SafeCast,
            },
            MsSqlType::Money => SafeCast,
            MsSqlType::SmallMoney => SafeCast,
            MsSqlType::Float(_) => SafeCast,
            MsSqlType::Real => SafeCast,
            MsSqlType::DateTime => SafeCast,
            MsSqlType::SmallDateTime => SafeCast,
            MsSqlType::Binary(_) => SafeCast,
            MsSqlType::VarBinary(_) => SafeCast,
            MsSqlType::Bit => RiskyCast,
            MsSqlType::Char(param) | MsSqlType::NChar(param) => match param {
                // TinyInt can be at most three digits, so this might fail.
                Some(len) if *len < 3 => RiskyCast,
                None => RiskyCast,
                _ => SafeCast,
            },
            MsSqlType::VarChar(param) | MsSqlType::NVarChar(param) => match param {
                // TinyInt can be at most three digits, so this might fail.
                Some(Number(len)) if *len < 3 => RiskyCast,
                None => RiskyCast,
                _ => SafeCast,
            },
            _ => NotCastable,
        },

        // Maps to i16.
        MsSqlType::SmallInt => match next {
            MsSqlType::TinyInt => RiskyCast,
            MsSqlType::SmallInt => SafeCast,
            MsSqlType::Int => SafeCast,
            MsSqlType::BigInt => SafeCast,
            MsSqlType::Decimal(params) => match params {
                // SmallInt can be at most five digits, so this might fail.
                Some((p, s)) if p - s < 5 => RiskyCast,
                _ => SafeCast,
            },
            MsSqlType::Money => SafeCast,
            MsSqlType::SmallMoney => SafeCast,
            MsSqlType::Float(_) => SafeCast,
            MsSqlType::Real => SafeCast,
            MsSqlType::DateTime => SafeCast,
            MsSqlType::SmallDateTime => SafeCast,
            MsSqlType::Binary(param) => match param {
                // SmallInt is two bytes, so this might fail.
                Some(n) if *n < 2 => RiskyCast,
                None => RiskyCast, // n == 1 by default
                _ => SafeCast,
            },
            MsSqlType::VarBinary(param) => match param {
                // SmallInt is two bytes, so this might fail.
                Some(Number(n)) if *n < 2 => RiskyCast,
                None => RiskyCast, // n == 1 by default
                _ => SafeCast,
            },
            MsSqlType::Bit => RiskyCast,
            MsSqlType::Char(param) | MsSqlType::NChar(param) => match param {
                // We can have five digits and an optional sign.
                Some(len) if *len < 6 => RiskyCast,
                None => RiskyCast,
                _ => SafeCast,
            },
            MsSqlType::VarChar(param) | MsSqlType::NVarChar(param) => match param {
                // We can have five digits and an optional sign.
                Some(Number(len)) if *len < 6 => RiskyCast,
                None => RiskyCast,
                _ => SafeCast,
            },
            _ => NotCastable,
        },

        // Maps to i32.
        MsSqlType::Int => match next {
            MsSqlType::TinyInt => RiskyCast,
            MsSqlType::SmallInt => RiskyCast,
            MsSqlType::Int => SafeCast,
            MsSqlType::BigInt => SafeCast,
            MsSqlType::Decimal(params) => match params {
                // Int can be at most ten digits, so this might fail.
                Some((p, s)) if p - s < 10 => RiskyCast,
                _ => SafeCast,
            },
            MsSqlType::Money => SafeCast,
            MsSqlType::SmallMoney => RiskyCast,
            MsSqlType::Float(_) => SafeCast,
            MsSqlType::Real => SafeCast,
            MsSqlType::DateTime => RiskyCast,
            MsSqlType::SmallDateTime => RiskyCast,
            MsSqlType::Binary(param) => match param {
                // Int is four bytes.
                Some(n) if *n < 4 => RiskyCast,
                None => RiskyCast, // n == 1 by default
                _ => SafeCast,
            },
            MsSqlType::VarBinary(param) => match param {
                // Int is four bytes.
                Some(Number(n)) if *n < 4 => RiskyCast,
                None => RiskyCast, // n == 1 by default
                _ => SafeCast,
            },
            MsSqlType::Bit => RiskyCast,
            MsSqlType::Char(param) | MsSqlType::NChar(param) => match param {
                // Int can be at most eleven characters, so this might fail.
                Some(len) if *len < 11 => RiskyCast,
                None => RiskyCast,
                _ => SafeCast,
            },
            MsSqlType::VarChar(param) | MsSqlType::NVarChar(param) => match param {
                // Int can be at most eleven characters, so this might fail.
                Some(Number(len)) if *len < 11 => RiskyCast,
                None => RiskyCast,
                _ => SafeCast,
            },
            _ => NotCastable,
        },

        // Maps to i64.
        MsSqlType::BigInt => match next {
            MsSqlType::TinyInt => RiskyCast,
            MsSqlType::SmallInt => RiskyCast,
            MsSqlType::Int => RiskyCast,
            MsSqlType::BigInt => SafeCast,
            MsSqlType::Decimal(params) => match params {
                // BigInt can have at most 19 digits.
                Some((p, s)) if p - s < 19 => RiskyCast,
                None => RiskyCast,
                _ => SafeCast,
            },
            MsSqlType::Money => RiskyCast,
            MsSqlType::SmallMoney => RiskyCast,
            MsSqlType::Float(_) => SafeCast,
            MsSqlType::Real => SafeCast,
            MsSqlType::DateTime => RiskyCast,
            MsSqlType::SmallDateTime => RiskyCast,
            MsSqlType::Binary(param) => match param {
                // BigInt is eight bytes.
                Some(n) if *n < 8 => RiskyCast,
                None => RiskyCast, // n == 1 by default
                _ => SafeCast,
            },
            MsSqlType::VarBinary(param) => match param {
                // BigInt is eight bytes.
                Some(Number(n)) if *n < 8 => RiskyCast,
                None => RiskyCast, // n == 1 by default
                _ => SafeCast,
            },
            MsSqlType::Bit => RiskyCast,
            MsSqlType::Char(param) | MsSqlType::NChar(param) => match param {
                // BigInt can have at most 20 characters.
                Some(len) if *len < 20 => RiskyCast,
                None => RiskyCast,
                _ => SafeCast,
            },
            MsSqlType::VarChar(param) | MsSqlType::NVarChar(param) => match param {
                // BigInt can have at most 20 characters.
                Some(Number(len)) if *len < 20 => RiskyCast,
                None => RiskyCast,
                _ => SafeCast,
            },
            _ => NotCastable,
        },

        // A number, described by precision and scale. Precision is the number
        // of digits in total we can have, scale the number of digits on the
        // right side of the comma.
        MsSqlType::Decimal(old_params) => {
            // todo most of these could be safe so we should match on the params as well?
            match next {
                MsSqlType::TinyInt => RiskyCast,
                MsSqlType::SmallInt => RiskyCast,
                MsSqlType::Int => RiskyCast,
                MsSqlType::BigInt => RiskyCast,
                MsSqlType::Money => RiskyCast,
                MsSqlType::SmallMoney => RiskyCast,
                MsSqlType::Bit => RiskyCast,
                MsSqlType::Float(_) => RiskyCast,
                MsSqlType::Real => RiskyCast,
                MsSqlType::DateTime => RiskyCast,
                MsSqlType::SmallDateTime => RiskyCast,
                MsSqlType::Binary(_) => RiskyCast,
                MsSqlType::VarBinary(_) => RiskyCast,
                MsSqlType::Decimal(new_params) => match (old_params, new_params) {
                    (Some((p_old, s_old)), None) if *p_old > 18 || *s_old > 0 => RiskyCast,
                    (None, Some((p_new, s_new))) if *p_new < 18 || *s_new > 0 => RiskyCast,
                    // Sigh... So, numeric(4,0) to numeric(4,2) would be risky,
                    // so would numeric(4,2) to numeric(4,0).
                    (Some((p_old, s_old)), Some((p_new, s_new))) if p_old - s_old > p_new - s_new || s_old > s_new => {
                        RiskyCast
                    }
                    _ => SafeCast,
                },
                MsSqlType::Char(length) | MsSqlType::NChar(length) => match (length, old_params) {
                    // We must fit p digits and a possible sign to our
                    // string, otherwise might truncate.
                    (Some(len), Some((p, 0))) if p + 1 > *len => RiskyCast,
                    // We must fit p digits, a possible sign and a comma to
                    // our string, otherwise might truncate.
                    (Some(len), Some((p, n))) if *n > 0 && p + 2 > *len => RiskyCast,
                    // Defaults to `number(18, 0)`, so we must fit 18 digits
                    // and a possible sign without truncating.
                    (Some(len), None) if *len < 19 => RiskyCast,
                    (None, _) => RiskyCast,
                    _ => SafeCast,
                },
                MsSqlType::VarChar(length) | MsSqlType::NVarChar(length) => match (length, old_params) {
                    // We must fit p digits and a possible sign to our
                    // string, otherwise might truncate.
                    (Some(Number(len)), Some((p, 0))) if p + 1 > (*len).into() => RiskyCast,
                    // We must fit p digits, a possible sign and a comma to
                    // our string, otherwise might truncate.
                    (Some(Number(len)), Some((p, n))) if *n > 0 && p + 2 > (*len).into() => RiskyCast,
                    // Defaults to `number(18, 0)`, so we must fit 18 digits
                    // and a possible sign without truncating.
                    (Some(Number(len)), None) if *len < 19 => RiskyCast,
                    (None, _) => RiskyCast,
                    _ => SafeCast,
                },
                _ => NotCastable,
            }
        }

        // A special number, with precision of 19 and scale of 4.
        MsSqlType::Money => match next {
            MsSqlType::TinyInt => RiskyCast,
            MsSqlType::SmallInt => RiskyCast,
            MsSqlType::Int => RiskyCast,
            MsSqlType::BigInt => RiskyCast,
            MsSqlType::Decimal(params) => match params {
                // We can have 19 digits and four decimals.
                Some((p, s)) if *p < 19 || *s < 4 => RiskyCast,
                None => RiskyCast,
                _ => SafeCast,
            },
            MsSqlType::Money => SafeCast,
            MsSqlType::SmallMoney => RiskyCast,
            MsSqlType::Bit => RiskyCast,
            MsSqlType::Float(_) => RiskyCast,
            MsSqlType::Real => RiskyCast,
            MsSqlType::DateTime => RiskyCast,
            MsSqlType::SmallDateTime => RiskyCast,
            MsSqlType::Char(param) | MsSqlType::NChar(param) => match param {
                // We can have 19 digits, comma and sign
                Some(len) if *len < 21 => RiskyCast,
                None => RiskyCast,
                _ => SafeCast,
            },
            MsSqlType::VarChar(param) | MsSqlType::NVarChar(param) => match param {
                // We can have 19 digits, comma and sign
                Some(Number(len)) if *len < 21 => RiskyCast,
                None => RiskyCast,
                _ => SafeCast,
            },
            MsSqlType::Binary(param) => match param {
                // Eight bytes.
                Some(len) if *len < 8 => RiskyCast,
                None => RiskyCast,
                _ => SafeCast,
            },
            MsSqlType::VarBinary(param) => match param {
                // Eight bytes.
                Some(Number(len)) if *len < 8 => RiskyCast,
                None => RiskyCast,
                _ => SafeCast,
            },
            _ => NotCastable,
        },

        // A special money number for poor people, with precision of 10 and
        // scale of 4.
        MsSqlType::SmallMoney => match next {
            MsSqlType::TinyInt => RiskyCast,
            MsSqlType::SmallInt => RiskyCast,
            MsSqlType::Int => RiskyCast,
            MsSqlType::BigInt => RiskyCast,
            MsSqlType::Decimal(params) => match params {
                // Ten digits, four decimals
                Some((p, s)) if *p < 10 || *s < 4 => RiskyCast,
                None => RiskyCast,
                _ => SafeCast,
            },
            MsSqlType::Money => SafeCast,
            MsSqlType::SmallMoney => SafeCast,
            MsSqlType::Bit => RiskyCast,
            MsSqlType::Float(_) => RiskyCast,
            MsSqlType::Real => RiskyCast,
            MsSqlType::DateTime => RiskyCast,
            MsSqlType::SmallDateTime => RiskyCast,
            MsSqlType::Char(param) | MsSqlType::NChar(param) => match param {
                // Ten digits, comma and a sign.
                Some(len) if *len < 12 => RiskyCast,
                None => RiskyCast,
                _ => SafeCast,
            },
            MsSqlType::VarChar(param) | MsSqlType::NVarChar(param) => match param {
                // Ten digits, comma and a sign.
                Some(Number(len)) if *len < 12 => RiskyCast,
                None => RiskyCast,
                _ => SafeCast,
            },
            MsSqlType::Binary(param) => match param {
                // Four bytes.
                Some(len) if *len < 4 => RiskyCast,
                None => RiskyCast,
                _ => SafeCast,
            },
            MsSqlType::VarBinary(param) => match param {
                // Four bytes.
                Some(Number(len)) if *len < 4 => RiskyCast,
                None => RiskyCast,
                _ => SafeCast,
            },
            _ => NotCastable,
        },

        // Either a float or double. Has a parameter, that very obviously is a
        // float when the parameter is 24 or less, a double when it's between 24
        // and 53.
        MsSqlType::Float(old_param) => match next {
            MsSqlType::TinyInt => RiskyCast,
            MsSqlType::SmallInt => RiskyCast,
            MsSqlType::Int => RiskyCast,
            MsSqlType::BigInt => RiskyCast,
            MsSqlType::Decimal(_) => RiskyCast,
            MsSqlType::Money => RiskyCast,
            MsSqlType::SmallMoney => RiskyCast,
            MsSqlType::Bit => RiskyCast,
            MsSqlType::Float(new_param) => match (old_param, new_param) {
                // If length is 24 or lower, we have a four byte float.
                (Some(left_len), Some(right_len)) if *left_len <= 24 && *right_len <= 24 => SafeCast,
                (Some(left_len), Some(right_len)) if *left_len > 24 && *right_len > 24 => SafeCast,
                // If length is not set, it's an eight byte float (double).
                (None, Some(right_len)) if *right_len > 24 => SafeCast,
                (Some(left_len), None) if *left_len > 24 => SafeCast,
                (None, None) => SafeCast,
                _ => RiskyCast,
            },
            MsSqlType::Real => match old_param {
                // Real is always a four byte float.
                Some(len) if *len <= 24 => SafeCast,
                _ => RiskyCast,
            },
            MsSqlType::SmallDateTime => RiskyCast,
            MsSqlType::DateTime => RiskyCast,
            MsSqlType::Char(new_param) | MsSqlType::NChar(new_param) => match (old_param, new_param) {
                // If float, we can have 47 characters including the sign and comma.
                (Some(f_len), Some(char_len)) if *f_len <= 24 && *char_len >= 47 => SafeCast,
                // If double, we can have 317 characters including the sign and comma.
                (Some(f_len), Some(char_len)) if *f_len > 24 && *char_len >= 317 => SafeCast,
                // If length not set, it's a double.
                (None, Some(char_len)) if *char_len >= 317 => SafeCast,
                _ => RiskyCast,
            },
            MsSqlType::VarChar(new_param) | MsSqlType::NVarChar(new_param) => match (old_param, new_param) {
                // If float, we can have 47 characters including the sign and comma.
                (Some(f_len), Some(Number(char_len))) if *f_len <= 24 && *char_len >= 47 => SafeCast,
                // If double, we can have 317 characters including the sign and comma.
                (Some(f_len), Some(Number(char_len))) if *f_len > 24 && *char_len >= 317 => SafeCast,
                // If length not set, it's a double.
                (None, Some(Number(char_len))) if *char_len >= 317 => SafeCast,
                _ => RiskyCast,
            },
            MsSqlType::Binary(new_param) => match (old_param, new_param) {
                // Float is four bytes.
                (Some(f_len), Some(bin_len)) if *f_len <= 24 && *bin_len >= 4 => SafeCast,
                // Double is eight bytes.
                (Some(f_len), Some(bin_len)) if *f_len > 24 && *bin_len >= 8 => SafeCast,
                // By default, we have a double.
                (None, Some(bin_len)) if *bin_len >= 8 => SafeCast,
                _ => RiskyCast,
            },
            MsSqlType::VarBinary(new_param) => match (old_param, new_param) {
                // Float is four bytes.
                (Some(f_len), Some(Number(bin_len))) if *f_len <= 24 && *bin_len >= 4 => SafeCast,
                // Double is eight bytes.
                (Some(f_len), Some(Number(bin_len))) if *f_len > 24 && *bin_len >= 8 => SafeCast,
                // By default, we have a double.
                (None, Some(Number(bin_len))) if *bin_len >= 8 => SafeCast,
                _ => RiskyCast,
            },
            _ => NotCastable,
        },

        // An alias for `float(24)`.
        MsSqlType::Real => match next {
            MsSqlType::TinyInt => RiskyCast,
            MsSqlType::SmallInt => RiskyCast,
            MsSqlType::Int => RiskyCast,
            MsSqlType::BigInt => RiskyCast,
            MsSqlType::Decimal(_) => RiskyCast,
            MsSqlType::Money => RiskyCast,
            MsSqlType::SmallMoney => RiskyCast,
            MsSqlType::Bit => RiskyCast,
            MsSqlType::Real => RiskyCast,
            MsSqlType::DateTime => RiskyCast,
            MsSqlType::SmallDateTime => RiskyCast,
            MsSqlType::Float(param) => match param {
                // Real is the same as float(24) or lower.
                Some(len) if *len <= 24 => SafeCast,
                _ => RiskyCast,
            },
            MsSqlType::Char(param) | MsSqlType::NChar(param) => match param {
                // We have 47 characters maximum.
                Some(len) if *len >= 47 => SafeCast,
                _ => RiskyCast,
            },
            MsSqlType::VarChar(param) | MsSqlType::NVarChar(param) => match param {
                // We have 47 characters maximum.
                Some(Number(len)) if *len >= 47 => SafeCast,
                _ => RiskyCast,
            },
            MsSqlType::Binary(param) => match param {
                // Real is four bytes.
                Some(len) if *len >= 4 => SafeCast,
                _ => RiskyCast,
            },
            MsSqlType::VarBinary(param) => match param {
                // Real is four bytes.
                Some(Number(len)) if *len >= 4 => SafeCast,
                _ => RiskyCast,
            },
            _ => NotCastable,
        },

        // Date values with no time.
        MsSqlType::Date => match next {
            MsSqlType::Date => SafeCast,
            MsSqlType::DateTime => SafeCast,
            MsSqlType::DateTime2 => SafeCast,
            MsSqlType::DateTimeOffset => SafeCast,
            MsSqlType::Char(param) | MsSqlType::NChar(param) => match param {
                // We can have 10 characters.
                Some(len) if *len >= 10 => SafeCast,
                _ => RiskyCast,
            },
            MsSqlType::VarChar(param) | MsSqlType::NVarChar(param) => match param {
                // We can have 10 characters.
                Some(Number(len)) if *len >= 10 => SafeCast,
                _ => RiskyCast,
            },
            MsSqlType::SmallDateTime => RiskyCast,
            _ => NotCastable,
        },

        // Time values with no date.
        MsSqlType::Time => match next {
            MsSqlType::Time => SafeCast,
            MsSqlType::DateTime => RiskyCast,
            MsSqlType::DateTime2 => SafeCast,
            MsSqlType::DateTimeOffset => SafeCast,
            MsSqlType::Char(param) | MsSqlType::NChar(param) => match param {
                // We can have 8 characters.
                Some(len) if *len >= 8 => SafeCast,
                _ => RiskyCast,
            },
            MsSqlType::VarChar(param) | MsSqlType::NVarChar(param) => match param {
                // We can have 8 characters.
                Some(Number(len)) if *len >= 8 => SafeCast,
                _ => RiskyCast,
            },
            MsSqlType::SmallDateTime => RiskyCast,
            _ => NotCastable,
        },

        // Date and time, in a precision of 1/300 seconds. Don't touch if you
        // don't have to.
        MsSqlType::DateTime => match next {
            MsSqlType::Date => RiskyCast,
            MsSqlType::Time => RiskyCast,
            MsSqlType::DateTime => SafeCast,
            MsSqlType::DateTime2 => SafeCast,
            MsSqlType::DateTimeOffset => SafeCast,
            MsSqlType::SmallDateTime => RiskyCast,
            MsSqlType::Char(param) | MsSqlType::NChar(param) => match param {
                // We can have 23 characters.
                Some(len) if *len >= 23 => SafeCast,
                _ => RiskyCast,
            },
            MsSqlType::VarChar(param) | MsSqlType::NVarChar(param) => match param {
                // We can have 23 characters.
                Some(Number(len)) if *len >= 23 => SafeCast,
                _ => RiskyCast,
            },
            _ => NotCastable,
        },

        // Date and time, in a precision of 100 nanoseconds. This is the type we
        // want for datetimes.
        MsSqlType::DateTime2 => match next {
            MsSqlType::Date => RiskyCast,
            MsSqlType::Time => RiskyCast,
            MsSqlType::DateTime => RiskyCast,
            MsSqlType::DateTime2 => SafeCast,
            MsSqlType::DateTimeOffset => SafeCast,
            MsSqlType::SmallDateTime => RiskyCast,
            MsSqlType::Char(param) | MsSqlType::NChar(param) => match param {
                // We can have 27 characters.
                Some(len) if *len >= 27 => SafeCast,
                _ => RiskyCast,
            },
            MsSqlType::VarChar(param) | MsSqlType::NVarChar(param) => match param {
                // We can have 27 characters.
                Some(Number(len)) if *len >= 27 => SafeCast,
                _ => RiskyCast,
            },
            _ => NotCastable,
        },

        // Datetime2 with an additional offset information (timezone).
        MsSqlType::DateTimeOffset => match next {
            MsSqlType::Date => RiskyCast,
            MsSqlType::Time => RiskyCast,
            MsSqlType::DateTime => RiskyCast,
            MsSqlType::DateTime2 => RiskyCast,
            MsSqlType::DateTimeOffset => SafeCast,
            MsSqlType::SmallDateTime => RiskyCast,
            MsSqlType::Char(param) | MsSqlType::NChar(param) => match param {
                // We can have 33 characters.
                Some(len) if *len >= 33 => SafeCast,
                _ => RiskyCast,
            },
            MsSqlType::VarChar(param) | MsSqlType::NVarChar(param) => match param {
                // We can have 33 characters.
                Some(Number(len)) if *len >= 33 => SafeCast,
                _ => RiskyCast,
            },
            _ => NotCastable,
        },

        // Like DateTime, but with less information. Only counts from 1.1.1900
        // upwards and doesn't hold seconds. Must be quite crazy to use this if
        // you don't have to.
        MsSqlType::SmallDateTime => match next {
            MsSqlType::Date => RiskyCast,
            MsSqlType::Time => RiskyCast,
            MsSqlType::DateTime => SafeCast,
            MsSqlType::DateTime2 => SafeCast,
            MsSqlType::DateTimeOffset => SafeCast,
            MsSqlType::SmallDateTime => SafeCast,
            MsSqlType::Char(param) | MsSqlType::NChar(param) => match param {
                // We can have 19 characters.
                Some(len) if *len >= 19 => SafeCast,
                _ => RiskyCast,
            },
            MsSqlType::VarChar(param) | MsSqlType::NVarChar(param) => match param {
                // We can have 19 characters.
                Some(Number(len)) if *len >= 19 => SafeCast,
                _ => RiskyCast,
            },
            _ => NotCastable,
        },

        // Text value taking a constant amount of space from the row. Convenient
        // if always having strings of same length in the column.
        //
        // ASCII only, except on SQL Server 2019 with UTF-8 collation.
        MsSqlType::Char(old_param) => match next {
            MsSqlType::TinyInt => RiskyCast,
            MsSqlType::SmallInt => RiskyCast,
            MsSqlType::Int => RiskyCast,
            MsSqlType::BigInt => RiskyCast,
            MsSqlType::Decimal(_) => RiskyCast,
            MsSqlType::Money => RiskyCast,
            MsSqlType::SmallMoney => RiskyCast,
            MsSqlType::Bit => RiskyCast,
            MsSqlType::Float(_) => RiskyCast,
            MsSqlType::Real => RiskyCast,
            MsSqlType::Date => RiskyCast,
            MsSqlType::Time => RiskyCast,
            MsSqlType::DateTime => RiskyCast,
            MsSqlType::DateTime2 => RiskyCast,
            MsSqlType::DateTimeOffset => RiskyCast,
            MsSqlType::SmallDateTime => RiskyCast,
            MsSqlType::Char(new_param) | MsSqlType::NChar(new_param) => match (old_param, new_param) {
                (Some(old_len), Some(new_len)) if old_len > new_len => RiskyCast,
                // Default length is 1.
                (Some(old_len), None) if *old_len > 1 => RiskyCast,
                _ => SafeCast,
            },
            MsSqlType::VarChar(new_param) | MsSqlType::NVarChar(new_param) => match (old_param, new_param) {
                (Some(old_len), Some(Number(new_len))) if *old_len > (*new_len).into() => RiskyCast,
                // Default length is 1.
                (Some(old_len), None) if *old_len > 1 => RiskyCast,
                _ => SafeCast,
            },
            MsSqlType::Text => SafeCast,
            MsSqlType::NText => SafeCast,
            MsSqlType::UniqueIdentifier => RiskyCast,
            _ => NotCastable,
        },

        // Text value taking a constant amount of space from the row. Convenient
        // if always having strings of same length in the column.
        //
        // UTF-16, uses always at least two bytes per character.
        MsSqlType::NChar(old_param) => match next {
            MsSqlType::TinyInt => RiskyCast,
            MsSqlType::SmallInt => RiskyCast,
            MsSqlType::Int => RiskyCast,
            MsSqlType::BigInt => RiskyCast,
            MsSqlType::Decimal(_) => RiskyCast,
            MsSqlType::Money => RiskyCast,
            MsSqlType::SmallMoney => RiskyCast,
            MsSqlType::Bit => RiskyCast,
            MsSqlType::Float(_) => RiskyCast,
            MsSqlType::Real => RiskyCast,
            MsSqlType::Date => RiskyCast,
            MsSqlType::Time => RiskyCast,
            MsSqlType::DateTime => RiskyCast,
            MsSqlType::DateTime2 => RiskyCast,
            MsSqlType::DateTimeOffset => RiskyCast,
            MsSqlType::SmallDateTime => RiskyCast,
            MsSqlType::NChar(new_param) => match (old_param, new_param) {
                (Some(old_len), Some(new_len)) if old_len > new_len => RiskyCast,
                // Default length is 1.
                (Some(old_len), None) if *old_len > 1 => RiskyCast,
                _ => SafeCast,
            },
            MsSqlType::Char(_) => RiskyCast,
            MsSqlType::VarChar(_) => RiskyCast,
            MsSqlType::NVarChar(new_param) => match (old_param, new_param) {
                (Some(old_len), Some(Number(new_len))) if *old_len > (*new_len).into() => RiskyCast,
                // Default length is 1.
                (Some(old_len), None) if *old_len > 1 => RiskyCast,
                _ => SafeCast,
            },
            MsSqlType::Text => RiskyCast,
            MsSqlType::NText => SafeCast,
            MsSqlType::UniqueIdentifier => RiskyCast,
            _ => NotCastable,
        },

        // Variable character, with a maximum length defined as a parameter. The
        // `Max` variant is stored outside of the row, and misses some of the
        // properties of a normal column; such as the primary key.
        //
        // ASCII only, except on SQL Server 2019 with UTF-8 collation.
        MsSqlType::VarChar(old_param) => match next {
            MsSqlType::TinyInt => RiskyCast,
            MsSqlType::SmallInt => RiskyCast,
            MsSqlType::Int => RiskyCast,
            MsSqlType::BigInt => RiskyCast,
            MsSqlType::Decimal(_) => RiskyCast,
            MsSqlType::Money => RiskyCast,
            MsSqlType::SmallMoney => RiskyCast,
            MsSqlType::Bit => RiskyCast,
            MsSqlType::Float(_) => RiskyCast,
            MsSqlType::Real => RiskyCast,
            MsSqlType::Date => RiskyCast,
            MsSqlType::Time => RiskyCast,
            MsSqlType::DateTime => RiskyCast,
            MsSqlType::DateTime2 => RiskyCast,
            MsSqlType::DateTimeOffset => RiskyCast,
            MsSqlType::SmallDateTime => RiskyCast,
            MsSqlType::NChar(new_param) | MsSqlType::Char(new_param) => match (old_param, new_param) {
                (Some(Number(old_len)), Some(new_len)) if u32::from(*old_len) > *new_len => RiskyCast,
                // Default length is 1.
                (Some(Number(old_len)), None) if *old_len > 1 => RiskyCast,
                (Some(Max), _) => RiskyCast,
                _ => SafeCast,
            },
            MsSqlType::NVarChar(Some(Max)) => match old_param {
                Some(Max) => RiskyCast,
                _ => SafeCast,
            },
            MsSqlType::VarChar(new_param) | MsSqlType::NVarChar(new_param) => match (old_param, new_param) {
                (Some(Number(old_len)), Some(Number(new_len))) if old_len > new_len => RiskyCast,
                // Default length is 1.
                (Some(Number(old_len)), None) if *old_len > 1 => RiskyCast,
                (Some(Max), Some(Number(_))) => RiskyCast,
                (Some(Max), None) => RiskyCast,
                _ => SafeCast,
            },
            MsSqlType::Text => SafeCast,
            MsSqlType::NText => match old_param {
                Some(Max) => RiskyCast,
                _ => SafeCast,
            },
            MsSqlType::UniqueIdentifier => RiskyCast,
            _ => NotCastable,
        },

        // Variable character, with a maximum length defined as a parameter. The
        // `Max` variant is stored outside of the row, and misses some of the
        // properties of a normal column; such as the primary key.
        //
        // UTF-16, uses always at least two bytes per character.
        MsSqlType::NVarChar(old_param) => match next {
            MsSqlType::TinyInt => RiskyCast,
            MsSqlType::SmallInt => RiskyCast,
            MsSqlType::Int => RiskyCast,
            MsSqlType::BigInt => RiskyCast,
            MsSqlType::Decimal(_) => RiskyCast,
            MsSqlType::Money => RiskyCast,
            MsSqlType::SmallMoney => RiskyCast,
            MsSqlType::Bit => RiskyCast,
            MsSqlType::Float(_) => RiskyCast,
            MsSqlType::Real => RiskyCast,
            MsSqlType::Date => RiskyCast,
            MsSqlType::Time => RiskyCast,
            MsSqlType::DateTime => RiskyCast,
            MsSqlType::DateTime2 => RiskyCast,
            MsSqlType::DateTimeOffset => RiskyCast,
            MsSqlType::SmallDateTime => RiskyCast,
            MsSqlType::Char(_) => RiskyCast,
            MsSqlType::VarChar(_) => RiskyCast,
            MsSqlType::NChar(new_param) => match (old_param, new_param) {
                (Some(Number(old_len)), Some(new_len)) if u32::from(*old_len) > *new_len => RiskyCast,
                // Default length is 1.
                (Some(Number(old_len)), None) if *old_len > 1 => RiskyCast,
                (Some(Max), _) => RiskyCast,
                _ => SafeCast,
            },
            MsSqlType::NVarChar(new_param) => match (old_param, new_param) {
                (Some(Number(old_len)), Some(Number(new_len))) if old_len > new_len => RiskyCast,
                // Default length is 1.
                (Some(Number(old_len)), None) if *old_len > 1 => RiskyCast,
                (Some(Max), Some(Number(_))) => RiskyCast,
                (Some(Max), None) => RiskyCast,
                _ => SafeCast,
            },
            MsSqlType::Text => RiskyCast,
            MsSqlType::NText => SafeCast,
            MsSqlType::UniqueIdentifier => RiskyCast,
            _ => NotCastable,
        },

        // Replaced by VarChar(Max). Deprecated.
        MsSqlType::Text => match next {
            MsSqlType::Char(_) => RiskyCast,
            MsSqlType::NChar(_) => RiskyCast,
            MsSqlType::VarChar(param) => match param {
                Some(Max) => SafeCast,
                _ => RiskyCast,
            },
            MsSqlType::Text => SafeCast,
            // NVarChar uses double the space, meaning we have half the amount
            // of characters available. This transformation might fail.
            MsSqlType::NVarChar(_) => RiskyCast,
            // NText uses double the space, meaning we have half the amount
            // of characters available. This transformation might fail.
            MsSqlType::NText => RiskyCast,
            _ => NotCastable,
        },

        // Replaced by NVarChar(Max). Deprecated.
        MsSqlType::NText => match next {
            MsSqlType::Char(_) => RiskyCast,
            MsSqlType::NChar(_) => RiskyCast,
            MsSqlType::VarChar(_) => RiskyCast,
            MsSqlType::Text => RiskyCast,
            MsSqlType::NVarChar(param) => match param {
                Some(Max) => SafeCast,
                _ => RiskyCast,
            },
            MsSqlType::NText => SafeCast,
            _ => NotCastable,
        },

        // Constant length binary data.
        MsSqlType::Binary(old_param) => match next {
            MsSqlType::TinyInt => match old_param {
                // One byte for tinyint.
                Some(len) if *len <= 1 => SafeCast,
                None => SafeCast,
                _ => RiskyCast,
            },
            MsSqlType::SmallInt => match old_param {
                // Two bytes for smallint.
                Some(len) if *len <= 2 => SafeCast,
                None => SafeCast,
                _ => RiskyCast,
            },
            MsSqlType::Int => match old_param {
                // Four bytes for int.
                Some(len) if *len <= 4 => SafeCast,
                None => SafeCast,
                _ => RiskyCast,
            },
            MsSqlType::BigInt => match old_param {
                // Eight bytes for bigint.
                Some(len) if *len <= 8 => SafeCast,
                None => SafeCast,
                _ => RiskyCast,
            },
            MsSqlType::Decimal(_) => RiskyCast,
            MsSqlType::Money => match old_param {
                // We can fit at most eight bytes here.
                Some(len) if *len > 8 => RiskyCast,
                _ => SafeCast,
            },
            MsSqlType::SmallMoney => match old_param {
                // We can fit at most four bytes here.
                Some(len) if *len > 4 => RiskyCast,
                _ => SafeCast,
            },
            MsSqlType::Bit => RiskyCast,
            MsSqlType::DateTime => RiskyCast,
            MsSqlType::SmallDateTime => RiskyCast,
            MsSqlType::Char(new_param) => match (old_param, new_param) {
                (Some(binary_len), Some(char_len)) if binary_len > char_len => RiskyCast,
                // Default Char length is one.
                (Some(binary_len), None) if *binary_len > 1 => RiskyCast,
                _ => SafeCast,
            },
            MsSqlType::NChar(new_param) => match (old_param, new_param) {
                // NChar uses twice the space per length unit.
                (Some(binary_len), Some(nchar_len)) if *binary_len > (nchar_len * 2) => RiskyCast,
                // By default we use two bytes.
                (Some(binary_len), None) if *binary_len > 2 => RiskyCast,
                _ => SafeCast,
            },
            MsSqlType::VarChar(new_param) => match (old_param, new_param) {
                (Some(binary_len), Some(Number(varchar_len))) if *binary_len > (*varchar_len).into() => RiskyCast,
                // By default we can fit one byte.
                (Some(binary_len), None) if *binary_len > 1 => RiskyCast,
                _ => SafeCast,
            },
            MsSqlType::NVarChar(new_param) => match (old_param, new_param) {
                // NVarChar takes double the space per length unit.
                (Some(binary_len), Some(Number(nvarchar_len))) if (*binary_len) > (*nvarchar_len * 2).into() => {
                    RiskyCast
                }
                // By default we can fit two bytes.
                (Some(binary_len), None) if *binary_len > 2 => RiskyCast,
                _ => SafeCast,
            },
            MsSqlType::Binary(new_param) => match (old_param, new_param) {
                (Some(old_len), Some(new_len)) if old_len > new_len => RiskyCast,
                // By default we can fit one byte.
                (Some(old_len), None) if *old_len > 1 => RiskyCast,
                _ => SafeCast,
            },
            MsSqlType::VarBinary(new_param) => match (old_param, new_param) {
                (Some(old_len), Some(Number(new_len))) if *old_len > (*new_len).into() => RiskyCast,
                // By default we can fit one byte.
                (Some(old_len), None) if *old_len > 1 => RiskyCast,
                _ => SafeCast,
            },
            MsSqlType::Image => SafeCast,
            MsSqlType::Xml => RiskyCast,
            MsSqlType::UniqueIdentifier => RiskyCast,
            _ => NotCastable,
        },

        // Variable length binary data. Has a Max variant for storage outside of
        // the row, allowing two gigabytes per column.
        MsSqlType::VarBinary(old_param) => match next {
            MsSqlType::TinyInt => match old_param {
                // One byte.
                Some(Number(binary_len)) if *binary_len <= 1 => SafeCast,
                None => SafeCast,
                _ => RiskyCast,
            },
            MsSqlType::SmallInt => match old_param {
                // Two bytes.
                Some(Number(binary_len)) if *binary_len <= 2 => SafeCast,
                _ => RiskyCast,
            },
            MsSqlType::Int => match old_param {
                // Four bytes.
                Some(Number(binary_len)) if *binary_len <= 4 => SafeCast,
                _ => RiskyCast,
            },
            MsSqlType::BigInt => match old_param {
                // Eight bytes.
                Some(Number(binary_len)) if *binary_len <= 8 => SafeCast,
                _ => RiskyCast,
            },
            MsSqlType::Decimal(_) => RiskyCast,
            MsSqlType::Money => match old_param {
                // Spending eight bytes for money.
                Some(Number(binary_len)) if *binary_len > 8 => RiskyCast,
                Some(Max) => RiskyCast,
                _ => SafeCast,
            },
            MsSqlType::SmallMoney => match old_param {
                // Four bytes for money.
                Some(Number(binary_len)) if *binary_len > 4 => RiskyCast,
                Some(Max) => RiskyCast,
                _ => SafeCast,
            },
            MsSqlType::Bit => RiskyCast,
            MsSqlType::DateTime => RiskyCast,
            MsSqlType::SmallDateTime => RiskyCast,
            MsSqlType::Char(new_param) => match (old_param, new_param) {
                (Some(Number(binary_len)), Some(char_len)) if u32::from(*binary_len) > *char_len => RiskyCast,
                (Some(Number(binary_len)), None) if *binary_len > 1 => RiskyCast,
                (Some(Max), _) => RiskyCast,
                _ => SafeCast,
            },
            MsSqlType::NChar(new_param) => match (old_param, new_param) {
                // NChar length unit is two bytes.
                (Some(Number(binary_len)), Some(nchar_len)) if u32::from(*binary_len) > (nchar_len * 2) => RiskyCast,
                // One nchar takes two bytes.
                (Some(Number(binary_len)), None) if *binary_len > 2 => RiskyCast,
                (Some(Max), _) => RiskyCast,
                _ => SafeCast,
            },
            MsSqlType::VarChar(new_param) => match (old_param, new_param) {
                (Some(Number(binary_len)), Some(Number(varchar_len))) if binary_len > varchar_len => RiskyCast,
                (Some(Number(binary_len)), None) if *binary_len > 1 => RiskyCast,
                (Some(Max), Some(Max)) => SafeCast,
                (Some(Max), _) => RiskyCast,
                _ => SafeCast,
            },
            MsSqlType::NVarChar(new_param) => match (old_param, new_param) {
                // NVarChar length unit is two bytes.
                (Some(Number(binary_len)), Some(Number(nvarchar_len))) if *binary_len > (nvarchar_len * 2) => RiskyCast,
                // One nvarchar takes two bytes.
                (Some(Number(binary_len)), None) if *binary_len > 2 => RiskyCast,
                (Some(Max), Some(Max)) => SafeCast,
                (Some(Max), _) => RiskyCast,
                _ => SafeCast,
            },
            MsSqlType::Binary(new_param) => match (old_param, new_param) {
                (Some(Number(old_len)), Some(new_len)) if u32::from(*old_len) > *new_len => RiskyCast,
                (Some(Number(old_len)), None) if *old_len > 1 => RiskyCast,
                (Some(Max), _) => RiskyCast,
                _ => SafeCast,
            },
            MsSqlType::VarBinary(new_param) => match (old_param, new_param) {
                (Some(Number(old_len)), Some(Number(new_len))) if old_len > new_len => RiskyCast,
                (Some(Number(old_len)), None) if *old_len > 1 => RiskyCast,
                (Some(Max), Some(Max)) => SafeCast,
                (Some(Max), _) => RiskyCast,
                _ => SafeCast,
            },
            MsSqlType::Image => SafeCast,
            MsSqlType::Xml => RiskyCast,
            MsSqlType::UniqueIdentifier => RiskyCast,
            _ => NotCastable,
        },

        // Replaced by VarBinary(Max). Deprecated.
        MsSqlType::Image => match next {
            MsSqlType::Binary(_) => RiskyCast,
            MsSqlType::VarBinary(param) => match param {
                Some(Max) => SafeCast,
                _ => RiskyCast,
            },
            MsSqlType::Image => SafeCast,
            _ => NotCastable,
        },

        // Structured XML data.
        MsSqlType::Xml => match next {
            MsSqlType::Char(_) => RiskyCast,
            MsSqlType::NChar(_) => RiskyCast,
            // We might lose some information if VarChar is not using UTF-8
            // collation.
            MsSqlType::VarChar(_) => RiskyCast,
            MsSqlType::NVarChar(param) => match param {
                Some(Max) => SafeCast,
                _ => RiskyCast,
            },
            MsSqlType::Binary(_) => RiskyCast,
            MsSqlType::VarBinary(_) => RiskyCast,
            MsSqlType::Xml => SafeCast,
            _ => NotCastable,
        },

        // GUID. The rest of the world calls this UUID.
        MsSqlType::UniqueIdentifier => match next {
            MsSqlType::Char(param) | MsSqlType::NChar(param) => match param {
                Some(length) if *length >= 36 => SafeCast,
                _ => RiskyCast,
            },
            MsSqlType::VarChar(param) | MsSqlType::NVarChar(param) => match param {
                Some(Number(length)) if *length >= 36 => SafeCast,
                Some(Max) => SafeCast,
                _ => RiskyCast,
            },
            MsSqlType::Binary(param) => match param {
                Some(length) if *length >= 16 => SafeCast,
                _ => RiskyCast,
            },
            MsSqlType::VarBinary(param) => match param {
                Some(Number(length)) if *length >= 16 => SafeCast,
                Some(Max) => SafeCast,
                _ => RiskyCast,
            },
            _ => NotCastable,
        },
    };

    match (previous, next) {
        (p, n) if p == n => None,
        // https://docs.microsoft.com/en-us/sql/t-sql/data-types/float-and-real-transact-sql?view=sql-server-ver16#syntax
        (MsSqlType::Float(Some(53)), MsSqlType::Float(None))
        | (MsSqlType::Float(None), MsSqlType::Float(Some(53)))
        | (MsSqlType::Float(Some(24)), MsSqlType::Real)
        | (MsSqlType::Real, MsSqlType::Float(Some(24))) => None,
        _ => Some(cast()),
    }
}

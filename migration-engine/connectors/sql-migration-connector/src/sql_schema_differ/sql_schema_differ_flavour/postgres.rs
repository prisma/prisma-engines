use super::SqlSchemaDifferFlavour;
use crate::{
    flavour::PostgresFlavour,
    pair::Pair,
    sql_migration::AlterEnum,
    sql_schema_differ::column::{ColumnDiffer, ColumnTypeChange},
    sql_schema_differ::SqlSchemaDiffer,
};
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

    fn column_type_change(&self, differ: &ColumnDiffer<'_>) -> Option<ColumnTypeChange> {
        // List to scalar
        if differ.previous.arity().is_list() && !differ.next.arity().is_list() {
            return match (differ.previous.column_type_family(), differ.next.column_type_family()) {
                (_, ColumnTypeFamily::String) => Some(ColumnTypeChange::SafeCast),
                (_, _) => Some(ColumnTypeChange::NotCastable),
            };
        }

        // Scalar to list
        if !differ.previous.arity().is_list() && differ.next.arity().is_list() {
            match (differ.previous.column_type_family(), differ.next.column_type_family()) {
                (ColumnTypeFamily::Decimal, ColumnTypeFamily::Decimal)
                | (ColumnTypeFamily::Float, ColumnTypeFamily::Float)
                | (ColumnTypeFamily::Decimal, ColumnTypeFamily::Float)
                | (ColumnTypeFamily::Float, ColumnTypeFamily::Decimal)
                | (ColumnTypeFamily::Binary, ColumnTypeFamily::Binary) => return Some(ColumnTypeChange::NotCastable),
                _ => (),
            }
        }

        if differ.previous.column_type_family() == differ.next.column_type_family() {
            return None;
        }

        match (differ.previous.column_type_family(), differ.next.column_type_family()) {
            (_, ColumnTypeFamily::String) => Some(ColumnTypeChange::SafeCast),
            (ColumnTypeFamily::String, ColumnTypeFamily::Int)
            | (ColumnTypeFamily::DateTime, ColumnTypeFamily::Float)
            | (ColumnTypeFamily::String, ColumnTypeFamily::Float) => Some(ColumnTypeChange::NotCastable),
            (_, _) => Some(ColumnTypeChange::RiskyCast),
        }
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
}

use super::SqlSchemaDifferFlavour;
use crate::{
    database_schema::SqlDatabaseSchema,
    flavour::PostgresFlavour,
    pair::Pair,
    sql_migration::{AlterEnum, SequenceChange, SequenceChanges, SqlMigrationStep},
    sql_schema_differ::{column::ColumnTypeChange, differ_database::DifferDatabase},
};
use enumflags2::BitFlags;
use native_types::{CockroachType, PostgresType};
use once_cell::sync::Lazy;
use regex::RegexSet;
use sql_schema_describer::{
    postgres::PostgresSchemaExt,
    walkers::{ColumnWalker, IndexWalker},
};

/// These can be tables or views, depending on the PostGIS version. In both cases, they should be ignored.
static POSTGIS_TABLES_OR_VIEWS: Lazy<RegexSet> = Lazy::new(|| {
    RegexSet::new(&[
        // PostGIS. Reference: https://postgis.net/docs/manual-1.4/ch04.html#id418599
        "(?i)^spatial_ref_sys$",
        "(?i)^geometry_columns$",
        "(?i)^geography_columns$",
    ])
    .unwrap()
});

// https://www.postgresql.org/docs/12/pgbuffercache.html
static EXTENSION_VIEWS: Lazy<RegexSet> = Lazy::new(|| RegexSet::new(&["(?i)^pg_buffercache$"]).unwrap());

impl SqlSchemaDifferFlavour for PostgresFlavour {
    fn can_alter_primary_keys(&self) -> bool {
        self.is_cockroachdb()
    }

    fn can_rename_foreign_key(&self) -> bool {
        true
    }

    fn column_autoincrement_changed(&self, columns: Pair<ColumnWalker<'_>>) -> bool {
        if self.is_cockroachdb() {
            return false;
        }

        columns.previous.is_autoincrement() != columns.next.is_autoincrement()
    }

    fn column_type_change(&self, columns: Pair<ColumnWalker<'_>>) -> Option<ColumnTypeChange> {
        // Handle the enum cases first.
        match columns.map(|col| col.column_type_family().as_enum()).into_tuple() {
            (Some(previous_enum), Some(next_enum)) if previous_enum == next_enum => return None,
            (Some(_), Some(_)) => return Some(ColumnTypeChange::NotCastable),
            (None, Some(_)) | (Some(_), None) => return Some(ColumnTypeChange::NotCastable),
            (None, None) => (),
        };

        if self.is_cockroachdb() {
            cockroach_column_type_change(columns)
        } else {
            postgres_column_type_change(columns)
        }
    }

    fn push_enum_steps(&self, steps: &mut Vec<SqlMigrationStep>, db: &DifferDatabase<'_>) {
        for enum_differ in db.enum_pairs() {
            let mut alter_enum = AlterEnum {
                id: enum_differ.enums.map(|e| e.id),
                created_variants: enum_differ.created_values().map(String::from).collect(),
                dropped_variants: enum_differ.dropped_values().map(String::from).collect(),
                previous_usages_as_default: Vec::new(), // this gets filled in later
            };

            if alter_enum.is_empty() {
                continue;
            }

            push_alter_enum_previous_usages_as_default(db, &mut alter_enum);
            steps.push(SqlMigrationStep::AlterEnum(alter_enum));
        }

        for enm in db.created_enums() {
            steps.push(SqlMigrationStep::CreateEnum(enm.id))
        }

        for enm in db.dropped_enums() {
            steps.push(SqlMigrationStep::DropEnum(enm.id))
        }
    }

    fn push_alter_sequence_steps(&self, steps: &mut Vec<SqlMigrationStep>, db: &DifferDatabase<'_>) {
        if !self.is_cockroachdb() {
            return;
        }

        let schemas: Pair<(&SqlDatabaseSchema, &PostgresSchemaExt)> = db
            .schemas()
            .map(|schema| (schema, schema.describer_schema.downcast_connector_data()));

        let sequence_pairs = db
            .all_column_pairs()
            .map(|cols| {
                schemas
                    .zip(cols)
                    .map(|((schema, ext), column_id)| (schema.walk(column_id), ext))
            })
            .filter_map(|cols| {
                cols.map(|(col, ext)| {
                    col.default()
                        .and_then(|d| d.as_sequence())
                        .and_then(|sequence_name| ext.get_sequence(sequence_name))
                })
                .transpose()
            });

        for pair in sequence_pairs {
            let prev = pair.previous.1;
            let next = pair.next.1;
            let mut changes: BitFlags<SequenceChange> = BitFlags::default();

            if prev.min_value != next.min_value {
                changes |= SequenceChange::MinValue;
            }

            if prev.max_value != next.max_value {
                changes |= SequenceChange::MaxValue;
            }

            if prev.start_value != next.start_value {
                changes |= SequenceChange::Start;
            }

            if prev.cache_size != next.cache_size {
                changes |= SequenceChange::Cache;
            }

            if prev.increment_by != next.increment_by {
                changes |= SequenceChange::Increment;
            }

            if !changes.is_empty() {
                steps.push(SqlMigrationStep::AlterSequence(
                    pair.map(|p| p.0 as u32),
                    SequenceChanges(changes),
                ));
            }
        }
    }

    fn indexes_match(&self, a: IndexWalker<'_>, b: IndexWalker<'_>) -> bool {
        let columns_previous = a.columns();
        let columns_next = b.columns();

        let pg_ext_previous: &PostgresSchemaExt = a.schema.downcast_connector_data();
        let pg_ext_next: &PostgresSchemaExt = b.schema.downcast_connector_data();

        let previous_algo = pg_ext_previous.index_algorithm(a.id);
        let next_algo = pg_ext_next.index_algorithm(b.id);

        columns_previous.len() == columns_next.len()
            && previous_algo == next_algo
            && columns_previous.zip(columns_next).all(|(col_a, col_b)| {
                let a_class = pg_ext_previous.get_opclass(col_a.id);
                let b_class = pg_ext_next.get_opclass(col_b.id);
                let a_kind = a_class.map(|c| &c.kind);
                let b_kind = b_class.map(|c| &c.kind);
                let a_is_default = a_class.map(|c| c.is_default).unwrap_or(false);
                let b_is_default = b_class.map(|c| c.is_default).unwrap_or(false);

                // the dml doesn't always have opclass defined if it's the
                // default.
                a_kind == b_kind || (a_class.is_none() && b_is_default) || (b_class.is_none() && a_is_default)
            })
    }

    fn indexes_should_be_recreated_after_column_drop(&self) -> bool {
        true
    }

    fn index_should_be_renamed(&self, pair: Pair<IndexWalker<'_>>) -> bool {
        // Implements correct comparison for truncated index names.
        let (previous_name, next_name) = pair.map(|idx| idx.name()).into_tuple();

        previous_name != next_name
    }

    fn set_tables_to_redefine(&self, db: &mut DifferDatabase<'_>) {
        if !self.is_cockroachdb() {
            return;
        }

        let id_gets_dropped = db
            .table_pairs()
            .filter(|tables| {
                tables.column_pairs().any(|columns| {
                    let type_change = self.column_type_change(columns);
                    let is_id = columns.previous.is_single_primary_key();

                    is_id && matches!(type_change, Some(ColumnTypeChange::NotCastable))
                }) || tables.dropped_columns().any(|col| col.is_single_primary_key())
            })
            .map(|t| t.table_ids());

        db.tables_to_redefine = id_gets_dropped.collect();
    }

    fn string_matches_bytes(&self, string: &str, bytes: &[u8]) -> bool {
        if !string.starts_with("\\x") || string.len() - 2 != bytes.len() * 2 || !string.is_ascii() {
            return false;
        }

        let string = &string[2..];

        bytes.iter().enumerate().all(|(idx, byte)| {
            let chars = &string[idx * 2..idx * 2 + 2];
            if let Ok(byte_from_string) = u8::from_str_radix(chars, 16) {
                byte_from_string == *byte
            } else {
                false
            }
        })
    }

    fn table_should_be_ignored(&self, table_name: &str) -> bool {
        POSTGIS_TABLES_OR_VIEWS.is_match(table_name)
    }

    fn view_should_be_ignored(&self, view_name: &str) -> bool {
        POSTGIS_TABLES_OR_VIEWS.is_match(view_name) || EXTENSION_VIEWS.is_match(view_name)
    }
}

fn cockroach_column_type_change(columns: Pair<ColumnWalker<'_>>) -> Option<ColumnTypeChange> {
    use ColumnTypeChange::*;

    let previous_type: Option<CockroachType> = columns.previous.column_native_type();
    let next_type: Option<CockroachType> = columns.next.column_native_type();
    let from_list_to_scalar = columns.previous.arity().is_list() && !columns.next.arity().is_list();
    let from_scalar_to_list = !columns.previous.arity().is_list() && columns.next.arity().is_list();

    match (previous_type, next_type) {
        (_, Some(CockroachType::String(None))) if from_list_to_scalar => Some(SafeCast),
        (_, Some(CockroachType::String(_))) if from_list_to_scalar => Some(RiskyCast),
        (_, Some(CockroachType::Char(_))) if from_list_to_scalar => Some(RiskyCast),
        (_, _) if from_scalar_to_list || from_list_to_scalar => Some(NotCastable),
        (Some(previous), Some(next)) => cockroach_native_type_change_riskyness(previous, next, columns),
        // Unsupported types will have None as Native type
        (None, Some(_)) => Some(RiskyCast),
        (Some(_), None) => Some(RiskyCast),
        (None, None)
            if columns.previous.column_type().full_data_type == columns.previous.column_type().full_data_type =>
        {
            None
        }
        (None, None) => Some(RiskyCast),
    }
}

// https://go.crdb.dev/issue-v/49329/v22.1
fn cockroach_native_type_change_riskyness(
    previous: CockroachType,
    next: CockroachType,
    columns: Pair<ColumnWalker<'_>>,
) -> Option<ColumnTypeChange> {
    let covered_by_index = columns
        .map(|col| col.is_part_of_secondary_index() || col.is_part_of_primary_key())
        .into_tuple()
        == (true, true);

    match (previous, next) {
        (CockroachType::Int4, CockroachType::String(None)) if !covered_by_index => Some(ColumnTypeChange::SafeCast),
        (CockroachType::Int8, CockroachType::Int4) if !covered_by_index => Some(ColumnTypeChange::RiskyCast),
        (previous, next) if previous == next => None,
        // Timestamp default precisions
        (CockroachType::Time(None), CockroachType::Time(Some(6)))
        | (CockroachType::Time(Some(6)), CockroachType::Time(None))
        | (CockroachType::Timetz(None), CockroachType::Timetz(Some(6)))
        | (CockroachType::Timetz(Some(6)), CockroachType::Timetz(None))
        | (CockroachType::Timestamptz(None), CockroachType::Timestamptz(Some(6)))
        | (CockroachType::Timestamptz(Some(6)), CockroachType::Timestamptz(None))
        | (CockroachType::Timestamp(None), CockroachType::Timestamp(Some(6)))
        | (CockroachType::Timestamp(Some(6)), CockroachType::Timestamp(None)) => None,
        _ => Some(ColumnTypeChange::NotCastable),
    }
}

fn postgres_column_type_change(columns: Pair<ColumnWalker<'_>>) -> Option<ColumnTypeChange> {
    use ColumnTypeChange::*;
    let previous_type: Option<PostgresType> = columns.previous.column_native_type();
    let next_type: Option<PostgresType> = columns.next.column_native_type();
    let from_list_to_scalar = columns.previous.arity().is_list() && !columns.next.arity().is_list();
    let from_scalar_to_list = !columns.previous.arity().is_list() && columns.next.arity().is_list();

    match (previous_type, next_type) {
        (_, Some(PostgresType::Text)) if from_list_to_scalar => Some(SafeCast),
        (_, Some(PostgresType::VarChar(None))) if from_list_to_scalar => Some(SafeCast),
        (_, Some(PostgresType::VarChar(_))) if from_list_to_scalar => Some(RiskyCast),
        (_, Some(PostgresType::Char(_))) if from_list_to_scalar => Some(RiskyCast),
        (_, _) if from_scalar_to_list || from_list_to_scalar => Some(NotCastable),
        (Some(previous), Some(next)) => postgres_native_type_change_riskyness(previous, next),
        // Unsupported types will have None as Native type
        (None, Some(_)) => Some(RiskyCast),
        (Some(_), None) => Some(RiskyCast),
        (None, None)
            if columns.previous.column_type().full_data_type == columns.previous.column_type().full_data_type =>
        {
            None
        }
        (None, None) => Some(RiskyCast),
    }
}

fn postgres_native_type_change_riskyness(previous: PostgresType, next: PostgresType) -> Option<ColumnTypeChange> {
    use native_types::PostgresType::*;
    use ColumnTypeChange::*;

    // varchar / varbit without param=> unlimited length
    // char / bit without param => length is 1
    let next_is_char = || matches!(next, Char(_));

    let cast = || {
        Some(match previous {
            PostgresType::Inet => match next {
                PostgresType::Citext | PostgresType::Text | PostgresType::VarChar(_) => ColumnTypeChange::SafeCast,
                _ => NotCastable,
            },
            PostgresType::Money => match next {
                PostgresType::Citext | PostgresType::Text | PostgresType::VarChar(_) => ColumnTypeChange::SafeCast,
                PostgresType::Decimal(_) => ColumnTypeChange::RiskyCast,
                _ => RiskyCast,
            },
            PostgresType::Citext => match next {
                PostgresType::Text => SafeCast,
                PostgresType::VarChar(_) => SafeCast,
                _ => RiskyCast,
            },
            PostgresType::Oid => match next {
                PostgresType::Text => SafeCast,
                PostgresType::VarChar(_) => SafeCast,
                PostgresType::BigInt | PostgresType::Integer => SafeCast,
                _ => NotCastable,
            },
            SmallInt => match next {
                Integer => SafeCast,
                BigInt => SafeCast,
                Decimal(params) => match params {
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
                Decimal(params) => match params {
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
                Decimal(params) => match params {
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
            Decimal(old_params) => match next {
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
                Decimal(new_params) => match (old_params, new_params) {
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
                    (Some(len), Some((p, n))) if n > 0 && p + 2 > len => RiskyCast,
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
                Decimal(_) => RiskyCast,
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
                Decimal(_) => RiskyCast,
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
                Text | VarChar(None) | Citext => SafeCast,
                VarChar(_) | Char(_) => RiskyCast,
                _ => NotCastable,
            },
            ByteA => match next {
                Text | VarChar(None) => SafeCast,
                VarChar(Some(length)) | Char(Some(length)) if length > 2 => RiskyCast,
                _ => NotCastable,
            },
            Timestamp(a) => match next {
                Text | VarChar(None) => SafeCast,
                Char(Some(len)) | VarChar(Some(len)) if len > 22 => SafeCast,
                PostgresType::Timestamp(None) => return None,
                PostgresType::Timestamp(Some(b)) if a.is_none() || a == Some(b) => return None,
                Timestamp(_) | Timestamptz(_) | Date | Time(_) | Timetz(_) => SafeCast,
                _ => NotCastable,
            },
            Timestamptz(a) => match next {
                Text | VarChar(None) => SafeCast,
                Char(Some(len)) | VarChar(Some(len)) if len > 27 => SafeCast,
                PostgresType::Timestamptz(None) => return None,
                PostgresType::Timestamptz(Some(b)) if a.is_none() || a == Some(b) => return None,
                Timestamp(_) | Timestamptz(_) | Date | Time(_) | Timetz(_) => SafeCast,
                _ => NotCastable,
            },
            Date => match next {
                Text | VarChar(None) => SafeCast,
                Char(Some(len)) | VarChar(Some(len)) if len > 27 => SafeCast,
                Timestamp(_) | Timestamptz(_) => SafeCast,
                _ => NotCastable,
            },
            Time(a) => match next {
                Text | VarChar(None) => SafeCast,
                Char(Some(len)) | VarChar(Some(len)) if len > 13 => SafeCast,
                PostgresType::Time(None) => return None,
                PostgresType::Time(Some(b)) if a.is_none() || a == Some(b) => return None,
                Timetz(_) => SafeCast,
                _ => NotCastable,
            },
            Timetz(a) => match next {
                Text | VarChar(None) => SafeCast,
                Char(Some(len)) | VarChar(Some(len)) if len > 18 => SafeCast,
                PostgresType::Timetz(None) => return None,
                PostgresType::Timetz(Some(b)) if a.is_none() || a == Some(b) => return None,
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
            Uuid => match next {
                Text | VarChar(None) => SafeCast,
                VarChar(Some(length)) | Char(Some(length)) if length > 31 => SafeCast,
                _ => NotCastable,
            },
            Xml => match next {
                Text | VarChar(None) => SafeCast,
                VarChar(_) | Char(_) => RiskyCast,
                _ => NotCastable,
            },
            Json => match next {
                Text | JsonB | VarChar(None) => SafeCast,
                VarChar(_) | Char(_) => RiskyCast,
                _ => NotCastable,
            },
            JsonB => match next {
                Text | Json | VarChar(None) => SafeCast,
                VarChar(_) | Char(_) => RiskyCast,
                _ => NotCastable,
            },
        })
    };

    if previous == next {
        None
    } else {
        cast()
    }
}

fn push_alter_enum_previous_usages_as_default(db: &DifferDatabase<'_>, alter_enum: &mut AlterEnum) {
    let mut previous_usages_as_default: Vec<(_, Option<_>)> = Vec::new();

    let enum_names = db.schemas().walk(alter_enum.id).map(|enm| enm.name());

    for table in db.dropped_tables() {
        for column in table
            .columns()
            .filter(|col| col.column_type_is_enum(enum_names.previous) && col.default().is_some())
        {
            previous_usages_as_default.push((column.id, None));
        }
    }

    for tables in db.table_pairs() {
        for column in tables
            .dropped_columns()
            .filter(|col| col.column_type_is_enum(enum_names.previous) && col.default().is_some())
        {
            previous_usages_as_default.push((column.id, None));
        }

        for columns in tables
            .column_pairs()
            .filter(|col| col.previous.column_type_is_enum(enum_names.previous) && col.previous.default().is_some())
        {
            let next_usage_as_default = Some(&columns.next)
                .filter(|col| col.column_type_is_enum(enum_names.next) && col.default().is_some())
                .map(|col| col.id);

            previous_usages_as_default.push((columns.previous.id, next_usage_as_default));
        }
    }

    alter_enum.previous_usages_as_default = previous_usages_as_default;
}

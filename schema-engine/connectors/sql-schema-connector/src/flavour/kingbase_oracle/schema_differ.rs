use crate::{
    migration_pair::MigrationPair,
    sql_migration::{AlterEnum, SqlMigrationStep},
    sql_schema_differ::{SqlSchemaDifferFlavour, column::ColumnTypeChange, differ_database::DifferDatabase},
};
use psl::builtin_connectors::{KingbaseOracleNumberArguments, KingbaseOracleType};
use sql_schema_describer::{
    ColumnTypeFamily,
    walkers::{IndexWalker, TableColumnWalker},
};

/// Oracle-compatible migration-diff rules for KingbaseES.
#[derive(Debug, Default)]
pub(crate) struct KingbaseOracleSchemaDifferFlavour;

impl SqlSchemaDifferFlavour for KingbaseOracleSchemaDifferFlavour {
    fn contains_table(&self, tables: &[String], namespace: Option<&str>, table_name: &str) -> bool {
        let Some(namespace) = namespace else {
            return tables.iter().any(|table| table == table_name);
        };
        let namespace = sql_schema_describer::kingbase_oracle::normalize_namespace(namespace);
        let suffix = format!(".{table_name}");
        tables.iter().any(|table| {
            table
                .strip_suffix(&suffix)
                .is_some_and(|schema| sql_schema_describer::kingbase_oracle::normalize_namespace(schema) == namespace)
        })
    }

    fn can_rename_foreign_key(&self) -> bool {
        true
    }

    fn primary_key_column_type_change_requires_recreation(
        &self,
        columns: MigrationPair<TableColumnWalker<'_>>,
    ) -> bool {
        // R2 can widen an existing identity in place, including its key.
        !uses_native_bigint_for_type_change(columns)
    }

    fn dbgenerated_defaults_match(
        &self,
        columns: MigrationPair<TableColumnWalker<'_>>,
        previous: &str,
        next: &str,
    ) -> bool {
        // Legacy NUMBER columns outside the model's precision range retain
        // physical native metadata even though their Prisma type is Unsupported.
        let known_unsupported_number = matches!(
            columns.previous.column_native_type::<KingbaseOracleType>(),
            Some(KingbaseOracleType::Number(_))
        ) && matches!(
            (columns.previous.column_type_family(), columns.next.column_type_family()),
            (ColumnTypeFamily::Unsupported(previous), ColumnTypeFamily::Unsupported(next)) if previous == next
        );
        previous.eq_ignore_ascii_case(next)
            || (known_unsupported_number
                || matches!(
                    columns.previous.column_type_family(),
                    ColumnTypeFamily::Decimal | ColumnTypeFamily::Float
                ) && matches!(
                    columns.next.column_type_family(),
                    ColumnTypeFamily::Decimal | ColumnTypeFamily::Float
                ))
                && sql_schema_describer::kingbase_oracle::numeric_default_expressions_match(previous, next)
    }

    fn column_type_change(&self, columns: MigrationPair<TableColumnWalker<'_>>) -> Option<ColumnTypeChange> {
        match (
            columns.previous.column_type_family_as_enum(),
            columns.next.column_type_family_as_enum(),
        ) {
            (Some(previous), Some(next)) => {
                // Variant changes are applied by AlterEnum; they do not cast
                // every referencing column (which may not exist in the live DB
                // when evaluating pending migrations against the shadow DB).
                if previous.name() == next.name() && previous.namespace() == next.namespace() {
                    return None;
                }

                return Some(ColumnTypeChange::RiskyCast);
            }
            (Some(_), None) | (None, Some(_)) => return Some(ColumnTypeChange::NotCastable),
            (None, None) => (),
        }

        // Live Unsupported columns can retain native metadata for a later
        // conversion to a supported type; model Unsupported fields have none.
        // Their exact SQL type descriptions still identify the same type.
        if matches!(
            (columns.previous.column_type_family(), columns.next.column_type_family()),
            (ColumnTypeFamily::Unsupported(previous), ColumnTypeFamily::Unsupported(next)) if previous == next
        ) {
            return None;
        }

        if columns.previous.column_type().native_type == columns.next.column_type().native_type
            && columns.previous.column_type().family == columns.next.column_type().family
        {
            return None;
        }

        match (
            columns.previous.column_native_type::<KingbaseOracleType>(),
            columns.next.column_native_type::<KingbaseOracleType>(),
        ) {
            (Some(previous), Some(next)) => oracle_type_change(*previous, *next),
            (None, None)
                if columns.previous.column_type().full_data_type == columns.next.column_type().full_data_type =>
            {
                None
            }
            _ => Some(ColumnTypeChange::RiskyCast),
        }
    }

    fn push_enum_steps(&self, steps: &mut Vec<SqlMigrationStep>, db: &DifferDatabase<'_>) {
        for enum_differ in db.enum_pairs() {
            let mut alter_enum = AlterEnum {
                id: enum_differ.enums.map(|enm| enm.id),
                created_variants: enum_differ.created_values().map(str::to_owned).collect(),
                dropped_variants: enum_differ.dropped_values().map(str::to_owned).collect(),
                previous_usages_as_default: Vec::new(),
            };

            if alter_enum.is_empty() {
                continue;
            }

            push_alter_enum_previous_usages_as_default(db, &mut alter_enum);
            steps.push(SqlMigrationStep::AlterEnum(alter_enum));
        }

        steps.extend(db.created_enums().map(|enm| SqlMigrationStep::CreateEnum(enm.id)));
        steps.extend(db.dropped_enums().map(|enm| SqlMigrationStep::DropEnum(enm.id)));
    }

    fn indexes_should_be_recreated_after_column_drop(&self) -> bool {
        true
    }

    fn index_should_be_renamed(&self, indexes: MigrationPair<IndexWalker<'_>>) -> bool {
        indexes.previous.name() != indexes.next.name()
    }

    fn should_recreate_fks_covered_by_deleted_indexes(&self) -> bool {
        true
    }
}

/// Removing or recreating an enum cannot keep defaults that reference the
/// previous enum type. Collect those columns so the renderer can drop the
/// defaults before the enum change and restore the still-valid ones after it.
fn push_alter_enum_previous_usages_as_default(db: &DifferDatabase<'_>, alter_enum: &mut AlterEnum) {
    let mut previous_usages_as_default: Vec<(_, Option<_>)> = Vec::new();
    let enum_names = db
        .schemas
        .walk(alter_enum.id)
        .map(|enm| (enm.namespace().to_owned(), enm.name().to_owned()));

    for table in db.dropped_tables() {
        for column in table.columns().filter(|column| {
            column
                .column_type_family_as_enum()
                .is_some_and(|enm| enm.namespace() == enum_names.previous.0 && enm.name() == enum_names.previous.1)
                && column.default().is_some()
        }) {
            previous_usages_as_default.push((column.id, None));
        }
    }

    for tables in db.table_pairs() {
        for column in tables.dropped_columns().filter(|column| {
            column
                .column_type_family_as_enum()
                .is_some_and(|enm| enm.namespace() == enum_names.previous.0 && enm.name() == enum_names.previous.1)
                && column.default().is_some()
        }) {
            previous_usages_as_default.push((column.id, None));
        }

        for columns in tables.column_pairs().filter(|columns| {
            columns
                .previous
                .column_type_family_as_enum()
                .is_some_and(|enm| enm.namespace() == enum_names.previous.0 && enm.name() == enum_names.previous.1)
                && columns.previous.default().is_some()
        }) {
            let next_usage_as_default = Some(&columns.next)
                .filter(|column| {
                    column
                        .column_type_family_as_enum()
                        .is_some_and(|enm| enm.namespace() == enum_names.next.0 && enm.name() == enum_names.next.1)
                        && column.default().is_some()
                })
                .map(|column| column.id);

            previous_usages_as_default.push((columns.previous.id, next_usage_as_default));
        }
    }

    alter_enum.previous_usages_as_default = previous_usages_as_default;
}

/// Keep existing native integers in the integer family when widening to the
/// default BigInt mapping. R2 requires this for IDENTITY and compatible foreign
/// keys. Ordinary NUMBER columns and custom precision keep their Oracle types.
pub(super) fn uses_native_bigint_for_type_change(columns: MigrationPair<TableColumnWalker<'_>>) -> bool {
    let ext: &sql_schema_describer::kingbase_oracle::KingbaseOracleSchemaExt =
        columns.previous.schema.downcast_connector_data();
    let identity = matches!(
        ext.column_generations.get(&columns.previous.id),
        Some(sql_schema_describer::kingbase_oracle::ColumnGeneration::Identity)
    );
    let native_integer = if identity {
        // Removing IDENTITY continues to use the ordinary target type.
        columns.previous.is_autoincrement() && columns.next.is_autoincrement()
    } else {
        matches!(
            columns.previous.column_type().full_data_type.as_str(),
            "int2" | "int4" | "int8" | "smallint" | "integer" | "bigint"
        )
    };
    native_integer
        && matches!(
            columns.previous.column_type_family(),
            ColumnTypeFamily::Int | ColumnTypeFamily::BigInt
        )
        && matches!(columns.next.column_type_family(), ColumnTypeFamily::BigInt)
        && matches!(
            columns.next.column_native_type::<KingbaseOracleType>(),
            Some(KingbaseOracleType::Number(
                KingbaseOracleNumberArguments::Precision(19) | KingbaseOracleNumberArguments::PrecisionAndScale(19, 0)
            ))
        )
}

fn oracle_type_change(previous: KingbaseOracleType, next: KingbaseOracleType) -> Option<ColumnTypeChange> {
    use ColumnTypeChange::*;

    if previous == next
        || timestamp_precision_is_equivalent(previous, next)
        || catalog_types_are_equivalent(previous, next)
    {
        return None;
    }

    if is_lob_or_json(previous) && !is_character_or_lob(next) || is_lob_or_json(next) && !is_character_or_lob(previous)
    {
        return Some(NotCastable);
    }

    match (previous, next) {
        (
            KingbaseOracleType::Number(_),
            KingbaseOracleType::Boolean
            | KingbaseOracleType::Date
            | KingbaseOracleType::Timestamp(_)
            | KingbaseOracleType::TimestampTz(_)
            | KingbaseOracleType::TimestampLocalTz(_),
        )
        | (
            KingbaseOracleType::Date
            | KingbaseOracleType::Timestamp(_)
            | KingbaseOracleType::TimestampTz(_)
            | KingbaseOracleType::TimestampLocalTz(_),
            KingbaseOracleType::Number(_),
        ) => Some(NotCastable),
        (
            KingbaseOracleType::Number(
                KingbaseOracleNumberArguments::Precision(precision)
                | KingbaseOracleNumberArguments::PrecisionAndScale(precision, 0),
            ),
            KingbaseOracleType::VarChar2(Some(length)),
        ) if length > precision => Some(SafeCast),
        (
            KingbaseOracleType::Date
            | KingbaseOracleType::Timestamp(_)
            | KingbaseOracleType::TimestampTz(_)
            | KingbaseOracleType::TimestampLocalTz(_),
            KingbaseOracleType::BinaryFloat | KingbaseOracleType::BinaryDouble | KingbaseOracleType::Float(_),
        ) => Some(NotCastable),
        (KingbaseOracleType::Number(previous), KingbaseOracleType::Number(next)) => {
            Some(if number_can_represent(next, previous) {
                SafeCast
            } else {
                RiskyCast
            })
        }
        (
            KingbaseOracleType::Char(previous) | KingbaseOracleType::VarChar2(previous),
            KingbaseOracleType::Char(next) | KingbaseOracleType::VarChar2(next),
        ) => Some(if length_can_represent(next, previous) {
            SafeCast
        } else {
            RiskyCast
        }),
        (
            KingbaseOracleType::NChar(previous) | KingbaseOracleType::NVarChar2(previous),
            KingbaseOracleType::NChar(next) | KingbaseOracleType::NVarChar2(next),
        ) => Some(if length_can_represent(next, previous) {
            SafeCast
        } else {
            RiskyCast
        }),
        (KingbaseOracleType::CharByte(previous), KingbaseOracleType::CharByte(next))
        | (KingbaseOracleType::VarChar2Byte(previous), KingbaseOracleType::VarChar2Byte(next)) => {
            Some(if next >= previous { SafeCast } else { RiskyCast })
        }
        (KingbaseOracleType::URowId(previous), KingbaseOracleType::URowId(next)) => {
            Some(if length_can_represent(next, previous) {
                SafeCast
            } else {
                RiskyCast
            })
        }
        (KingbaseOracleType::Date, KingbaseOracleType::Timestamp(_) | KingbaseOracleType::TimestampTz(_)) => {
            Some(SafeCast)
        }
        (KingbaseOracleType::Timestamp(_) | KingbaseOracleType::TimestampTz(_), KingbaseOracleType::Date) => {
            Some(RiskyCast)
        }
        _ => Some(RiskyCast),
    }
}

/// Kingbase's PostgreSQL-compatible catalog canonicalizes several Oracle-mode
/// declarations. Treat those representations as equal during schema diffing so
/// a schema push does not emit a MODIFY on every subsequent run.
fn catalog_types_are_equivalent(previous: KingbaseOracleType, next: KingbaseOracleType) -> bool {
    use KingbaseOracleType::*;

    match (previous, next) {
        (
            Number(KingbaseOracleNumberArguments::Precision(previous)),
            Number(KingbaseOracleNumberArguments::PrecisionAndScale(next, 0)),
        )
        | (
            Number(KingbaseOracleNumberArguments::PrecisionAndScale(previous, 0)),
            Number(KingbaseOracleNumberArguments::Precision(next)),
        ) => previous == next,
        (Float(None | Some(25..=53)), BinaryDouble) | (BinaryDouble, Float(None | Some(25..=53))) => true,
        (Float(Some(1..=24)), BinaryFloat) | (BinaryFloat, Float(Some(1..=24))) => true,
        (Char(previous), Char(next))
        | (Char(previous), NChar(next))
        | (NChar(previous), Char(next))
        | (NChar(previous), NChar(next)) => previous.unwrap_or(1) == next.unwrap_or(1),
        (VarChar2(previous), VarChar2(next))
        | (VarChar2(previous), NVarChar2(next))
        | (NVarChar2(previous), VarChar2(next))
        | (NVarChar2(previous), NVarChar2(next)) => previous == next,
        (TimestampLocalTz(previous), Timestamp(next)) | (Timestamp(next), TimestampLocalTz(previous)) => {
            previous == next
        }
        _ => false,
    }
}

fn timestamp_precision_is_equivalent(previous: KingbaseOracleType, next: KingbaseOracleType) -> bool {
    matches!(
        (previous, next),
        (
            KingbaseOracleType::Timestamp(None),
            KingbaseOracleType::Timestamp(Some(6))
        ) | (
            KingbaseOracleType::Timestamp(Some(6)),
            KingbaseOracleType::Timestamp(None)
        ) | (
            KingbaseOracleType::TimestampTz(None),
            KingbaseOracleType::TimestampTz(Some(6))
        ) | (
            KingbaseOracleType::TimestampTz(Some(6)),
            KingbaseOracleType::TimestampTz(None)
        ) | (
            KingbaseOracleType::TimestampLocalTz(None),
            KingbaseOracleType::TimestampLocalTz(Some(6))
        ) | (
            KingbaseOracleType::TimestampLocalTz(Some(6)),
            KingbaseOracleType::TimestampLocalTz(None)
        )
    )
}

fn number_can_represent(target: KingbaseOracleNumberArguments, source: KingbaseOracleNumberArguments) -> bool {
    use KingbaseOracleNumberArguments::*;

    match (source, target) {
        (_, Unspecified) => true,
        (Unspecified, _) => false,
        (Precision(source_precision), Precision(target_precision)) => target_precision >= source_precision,
        (Precision(source_precision), PrecisionAndScale(target_precision, 0)) => target_precision >= source_precision,
        (PrecisionAndScale(source_precision, source_scale), Precision(target_precision)) => {
            source_scale == 0 && target_precision >= source_precision
        }
        (PrecisionAndScale(source_precision, source_scale), PrecisionAndScale(target_precision, target_scale)) => {
            target_precision.saturating_sub(target_scale) >= source_precision.saturating_sub(source_scale)
                && target_scale >= source_scale
        }
        (Precision(_), PrecisionAndScale(_, _)) => false,
    }
}

fn length_can_represent(next: Option<u32>, previous: Option<u32>) -> bool {
    match (previous, next) {
        (_, None) => true,
        (None, Some(_)) => false,
        (Some(previous), Some(next)) => next >= previous,
    }
}

fn is_lob_or_json(native_type: KingbaseOracleType) -> bool {
    matches!(
        native_type,
        KingbaseOracleType::Blob | KingbaseOracleType::Clob | KingbaseOracleType::NClob | KingbaseOracleType::Json
    )
}

fn is_character_or_lob(native_type: KingbaseOracleType) -> bool {
    matches!(
        native_type,
        KingbaseOracleType::Char(_)
            | KingbaseOracleType::VarChar2(_)
            | KingbaseOracleType::NChar(_)
            | KingbaseOracleType::NVarChar2(_)
            | KingbaseOracleType::CharByte(_)
            | KingbaseOracleType::VarChar2Byte(_)
            | KingbaseOracleType::URowId(_)
            | KingbaseOracleType::Clob
            | KingbaseOracleType::NClob
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn unsupported_numeric_defaults_require_known_physical_number_metadata() {
        use psl::datamodel_connector::NativeTypeInstance;
        use sql_schema_describer::{Column, ColumnArity, ColumnType, SqlSchema};

        let column_schema = |family: ColumnTypeFamily, native: Option<KingbaseOracleType>| {
            let mut schema = SqlSchema::default();
            let namespace = schema.push_namespace("public".to_owned());
            let table = schema.push_table("Product".to_owned(), namespace, None);
            schema.push_table_column(
                table,
                Column {
                    name: "price".to_owned(),
                    tpe: ColumnType {
                        native_type: native.map(NativeTypeInstance::new),
                        ..ColumnType::pure(family, ColumnArity::Nullable)
                    },
                    auto_increment: false,
                    description: None,
                },
            );
            schema
        };
        let number = KingbaseOracleType::Number(KingbaseOracleNumberArguments::PrecisionAndScale(65, 30));
        let unsupported = ColumnTypeFamily::Unsupported("numeric(65,30)".to_owned());
        for native in [Some(number), None, Some(KingbaseOracleType::VarChar2(Some(100)))] {
            let previous = column_schema(unsupported.clone(), native);
            let next = column_schema(unsupported.clone(), None);
            let columns = MigrationPair::new(
                previous.walk_table_columns().next().unwrap(),
                next.walk_table_columns().next().unwrap(),
            );
            assert_eq!(
                KingbaseOracleSchemaDifferFlavour.dbgenerated_defaults_match(
                    columns,
                    "1.25::numeric(10,1)",
                    "CAST(1.25 AS NUMBER(10,1))"
                ),
                native == Some(number)
            );
            assert!(!KingbaseOracleSchemaDifferFlavour.dbgenerated_defaults_match(
                columns,
                "1.25::numeric(10,1)",
                "CAST(1.25 AS NUMBER(10,2))"
            ));
        }
        let previous = column_schema(unsupported, Some(number));
        let next = column_schema(ColumnTypeFamily::Unsupported("custom_domain".to_owned()), None);
        assert!(!KingbaseOracleSchemaDifferFlavour.dbgenerated_defaults_match(
            MigrationPair::new(
                previous.walk_table_columns().next().unwrap(),
                next.walk_table_columns().next().unwrap()
            ),
            "1.25::numeric(10,1)",
            "CAST(1.25 AS NUMBER(10,1))",
        ));
    }

    #[test]
    fn number_scale_above_precision_can_be_widened_without_underflow() {
        use KingbaseOracleNumberArguments::PrecisionAndScale;
        use KingbaseOracleType::Number;
        assert_eq!(
            oracle_type_change(Number(PrecisionAndScale(38, 50)), Number(PrecisionAndScale(38, 60))),
            Some(ColumnTypeChange::SafeCast)
        );
        assert_eq!(
            oracle_type_change(Number(PrecisionAndScale(38, 60)), Number(PrecisionAndScale(38, 50))),
            Some(ColumnTypeChange::RiskyCast)
        );
    }

    #[test]
    fn omitted_native_arguments_match_only_their_actual_defaults() {
        use KingbaseOracleNumberArguments::{Precision, PrecisionAndScale, Unspecified};
        use KingbaseOracleType::*;
        for (short, explicit) in [
            (Number(Precision(10)), Number(PrecisionAndScale(10, 0))),
            (Number(Precision(19)), Number(PrecisionAndScale(19, 0))),
            (Char(None), Char(Some(1))),
            (NChar(None), NChar(Some(1))),
        ] {
            assert_eq!(oracle_type_change(short, explicit), None);
            assert_eq!(oracle_type_change(explicit, short), None);
        }
        for (previous, next) in [
            (Number(Precision(10)), Number(PrecisionAndScale(11, 0))),
            (Number(Precision(10)), Number(PrecisionAndScale(10, 1))),
            (Number(Unspecified), Number(PrecisionAndScale(10, 0))),
            (Char(None), Char(Some(2))),
            (NChar(None), NChar(Some(2))),
        ] {
            assert!(oracle_type_change(previous, next).is_some(), "{previous:?} / {next:?}");
        }
    }

    #[test]
    fn external_table_schema_aliases_preserve_mixed_case() {
        use crate::sql_schema_differ::SqlSchemaDifferFlavour;
        let flavour = super::KingbaseOracleSchemaDifferFlavour;
        let tables = vec!["APP.Item".to_owned(), "Other.App.Item".to_owned()];
        assert!(flavour.contains_table(&tables, Some("app"), "Item"));
        assert!(flavour.contains_table(&tables, Some("APP"), "Item"));
        assert!(!flavour.contains_table(&tables, Some("App"), "Item"));
        assert!(flavour.contains_table(&tables, Some("Other.App"), "Item"));
        assert!(!flavour.contains_table(&tables, Some("app"), "item"));
    }

    use super::*;

    #[test]
    fn numeric_cast_defaults_compare_by_meaning_only_on_oracle_numeric_columns() {
        use psl::parser_database::NoExtensionTypes;
        use schema_connector::SchemaDialect;

        for (provider, dialect) in [
            ("kingbase-oracle", crate::SqlSchemaDialect::kingbase_oracle()),
            #[cfg(feature = "postgresql")]
            ("postgresql", crate::SqlSchemaDialect::postgres()),
            #[cfg(feature = "mysql")]
            ("mysql", crate::SqlSchemaDialect::mysql()),
        ] {
            for scalar in ["Decimal", "Float", "String"] {
                let calculate = |expression: &str| {
                    dialect.schema_from_datamodel(
                        vec![("schema.prisma".into(), format!(
                            "datasource db {{\n provider = \"{provider}\"\n }}\nmodel Product {{\n id Int @id\n price {scalar} @default(dbgenerated({}))\n }}",
                            serde_json::to_string(expression).unwrap()
                        ).into())], None, &NoExtensionTypes,
                    ).unwrap()
                };
                for expression in ["1.25::numeric(10,1)", "(1.25::numeric(3,2))::numeric(10,1)"] {
                    let migration = dialect.diff(
                        calculate(expression),
                        calculate("CAST(1.25 AS NUMBER(10,1))"),
                        &Default::default(),
                    );
                    let sql = dialect.render_script(&migration, &Default::default()).unwrap();
                    assert_eq!(
                        sql == "-- This is an empty migration.",
                        provider == "kingbase-oracle" && scalar != "String",
                        "{provider}/{scalar}: {sql}",
                    );
                }
                for expression in ["CAST(1.25 AS NUMBER(10,2))", "CAST(1.35 AS NUMBER(10,1))"] {
                    let migration = dialect.diff(
                        calculate("1.25::numeric(10,1)"),
                        calculate(expression),
                        &Default::default(),
                    );
                    let sql = dialect.render_script(&migration, &Default::default()).unwrap();
                    assert_ne!(sql, "-- This is an empty migration.", "{provider}/{scalar}");
                    assert!(sql.contains(expression), "{sql}");
                }
            }
        }
    }

    #[test]
    fn unsupported_implicit_conversions_require_column_recreation() {
        let number = KingbaseOracleType::Number(KingbaseOracleNumberArguments::PrecisionAndScale(10, 0));
        for (previous, next) in [
            (number, KingbaseOracleType::Boolean),
            (number, KingbaseOracleType::Timestamp(Some(3))),
            (KingbaseOracleType::Timestamp(Some(3)), number),
        ] {
            assert_eq!(oracle_type_change(previous, next), Some(ColumnTypeChange::NotCastable));
        }
    }
    #[test]
    fn integer_to_string_requires_room_for_all_digits_and_sign() {
        let integer = KingbaseOracleType::Number(KingbaseOracleNumberArguments::PrecisionAndScale(10, 0));
        assert_eq!(
            oracle_type_change(integer, KingbaseOracleType::VarChar2(Some(11))),
            Some(ColumnTypeChange::SafeCast)
        );
        assert_eq!(
            oracle_type_change(integer, KingbaseOracleType::VarChar2(Some(10))),
            Some(ColumnTypeChange::RiskyCast)
        );
        assert_eq!(
            oracle_type_change(
                KingbaseOracleType::Number(KingbaseOracleNumberArguments::PrecisionAndScale(10, 2)),
                KingbaseOracleType::VarChar2(Some(4000)),
            ),
            Some(ColumnTypeChange::RiskyCast)
        );
    }

    #[test]
    fn temporal_values_cannot_be_implicitly_cast_to_floats() {
        for previous in [
            KingbaseOracleType::Date,
            KingbaseOracleType::Timestamp(Some(3)),
            KingbaseOracleType::TimestampTz(Some(3)),
            KingbaseOracleType::TimestampLocalTz(Some(3)),
        ] {
            for next in [
                KingbaseOracleType::BinaryFloat,
                KingbaseOracleType::BinaryDouble,
                KingbaseOracleType::Float(None),
            ] {
                assert_eq!(oracle_type_change(previous, next), Some(ColumnTypeChange::NotCastable));
            }
        }
    }

    #[test]
    fn only_widening_number_and_character_changes_are_safe() {
        assert_eq!(
            oracle_type_change(
                KingbaseOracleType::Number(KingbaseOracleNumberArguments::PrecisionAndScale(10, 2)),
                KingbaseOracleType::Number(KingbaseOracleNumberArguments::PrecisionAndScale(12, 2))
            ),
            Some(ColumnTypeChange::SafeCast)
        );
        assert_eq!(
            oracle_type_change(
                KingbaseOracleType::VarChar2(Some(40)),
                KingbaseOracleType::VarChar2(Some(30))
            ),
            Some(ColumnTypeChange::RiskyCast)
        );

        assert_eq!(
            oracle_type_change(
                KingbaseOracleType::Number(KingbaseOracleNumberArguments::PrecisionAndScale(12, 4)),
                KingbaseOracleType::Number(KingbaseOracleNumberArguments::Precision(12))
            ),
            Some(ColumnTypeChange::RiskyCast)
        );

        assert_eq!(
            oracle_type_change(
                KingbaseOracleType::Number(KingbaseOracleNumberArguments::PrecisionAndScale(12, 0)),
                KingbaseOracleType::Number(KingbaseOracleNumberArguments::Precision(12))
            ),
            None
        );
    }

    #[test]
    fn lob_to_non_character_types_are_not_castable() {
        assert_eq!(
            oracle_type_change(
                KingbaseOracleType::Blob,
                KingbaseOracleType::Number(KingbaseOracleNumberArguments::Precision(10))
            ),
            Some(ColumnTypeChange::NotCastable)
        );
    }

    #[test]
    fn catalog_canonicalized_oracle_types_are_idempotent() {
        assert_eq!(
            oracle_type_change(KingbaseOracleType::NChar(Some(10)), KingbaseOracleType::Char(Some(10))),
            None
        );
        assert_eq!(
            oracle_type_change(
                KingbaseOracleType::NVarChar2(Some(32)),
                KingbaseOracleType::VarChar2(Some(32))
            ),
            None
        );
        assert_eq!(
            oracle_type_change(KingbaseOracleType::NClob, KingbaseOracleType::Clob),
            Some(ColumnTypeChange::RiskyCast)
        );
        assert_eq!(
            oracle_type_change(KingbaseOracleType::Timestamp(Some(0)), KingbaseOracleType::Date),
            Some(ColumnTypeChange::RiskyCast)
        );
        assert_eq!(
            oracle_type_change(
                KingbaseOracleType::Timestamp(Some(3)),
                KingbaseOracleType::TimestampLocalTz(Some(3))
            ),
            None
        );
        assert_eq!(
            oracle_type_change(KingbaseOracleType::BinaryDouble, KingbaseOracleType::Float(None)),
            None
        );
    }
    #[test]
    fn distinct_physical_types_are_not_hidden_by_catalog_alias_rules() {
        assert_eq!(
            oracle_type_change(KingbaseOracleType::Date, KingbaseOracleType::Timestamp(None)),
            Some(ColumnTypeChange::SafeCast)
        );
        assert_eq!(
            oracle_type_change(KingbaseOracleType::Clob, KingbaseOracleType::NClob),
            Some(ColumnTypeChange::RiskyCast)
        );
        assert_eq!(
            oracle_type_change(KingbaseOracleType::BinaryDouble, KingbaseOracleType::Float(Some(1))),
            Some(ColumnTypeChange::RiskyCast)
        );
        assert_eq!(
            oracle_type_change(KingbaseOracleType::BinaryFloat, KingbaseOracleType::Float(Some(24))),
            None
        );
    }
}

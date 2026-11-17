use crate::migration_pair::MigrationPair;
use psl::builtin_connectors::KingbaseMySqlType;
use sql_schema_describer::{
    ColumnTypeFamily,
    walkers::{IndexWalker, TableColumnWalker},
};

use crate::sql_schema_differ::{SqlSchemaDifferFlavour, all_match, column::ColumnTypeChange};

/// Whether a type change can be represented by an explicit Kingbase `USING`
/// expression. This is deliberately separate from whether Kingbase performs
/// the conversion implicitly: the renderer uses this information to keep
/// executable conversions as ALTER COLUMN operations instead of planning a
/// destructive drop/recreate.
pub(crate) fn explicit_cast_supported(previous: Option<&KingbaseMySqlType>, next: Option<&KingbaseMySqlType>) -> bool {
    let (Some(previous), Some(next)) = (previous, next) else {
        return false;
    };

    if is_blob(previous) && !is_blob(next) {
        // BLOB values can be decoded as UTF-8 text, but there is no reliable
        // type-level conversion for arbitrary binary data to numeric, temporal,
        // or JSON values. Those changes must retain the destructive path.
        return is_string(next) || (is_binary(next) && !is_bit(next));
    }

    if is_json(previous) && (is_numeric(next) || is_datetime(next) || is_bit(next)) {
        return false;
    }

    if is_binary(previous) && (is_datetime(next) || is_json(next) || is_numeric(next) || is_bit(next)) {
        return false;
    }

    if is_numeric(previous) && is_datetime(next) {
        return false;
    }

    if is_numeric(previous) && matches!(next, KingbaseMySqlType::Year) {
        return false;
    }

    if is_datetime(previous) && (is_json(next) || is_bit(next)) {
        return false;
    }

    // Numeric-to-JSON uses to_json() in the renderer;
    // all remaining native families have an explicit cast
    // representation in the Kingbase type system.
    true
}

#[derive(Debug, Default)]
pub(crate) struct KingbaseMysqlSchemaDifferFlavour;

impl SqlSchemaDifferFlavour for KingbaseMysqlSchemaDifferFlavour {
    fn can_rename_foreign_key(&self) -> bool {
        false
    }

    fn can_rename_index(&self) -> bool {
        true
    }

    fn can_cope_with_foreign_key_column_becoming_non_nullable(&self) -> bool {
        false
    }

    fn dbgenerated_defaults_match(
        &self,
        columns: MigrationPair<TableColumnWalker<'_>>,
        previous: &str,
        next: &str,
    ) -> bool {
        previous.eq_ignore_ascii_case(next)
            || matches!(
                columns.previous.column_type_family(),
                ColumnTypeFamily::Decimal | ColumnTypeFamily::Float
            ) && matches!(
                columns.next.column_type_family(),
                ColumnTypeFamily::Decimal | ColumnTypeFamily::Float
            ) && sql_schema_describer::kingbase_mysql::numeric_default_expressions_match(previous, next)
    }

    fn primary_key_column_type_change_requires_recreation(
        &self,
        columns: MigrationPair<TableColumnWalker<'_>>,
    ) -> bool {
        // R3 rejects dropping the key on an identity column. MODIFY can widen
        // INT to BIGINT while keeping the existing key and its dependencies.
        !(columns.previous.is_autoincrement()
            && matches!(
                (
                    columns.previous.column_native_type::<KingbaseMySqlType>(),
                    columns.next.column_native_type::<KingbaseMySqlType>()
                ),
                (Some(KingbaseMySqlType::Int), Some(KingbaseMySqlType::BigInt))
            ))
    }

    fn column_type_change(&self, differ: MigrationPair<TableColumnWalker<'_>>) -> Option<ColumnTypeChange> {
        match (
            differ.previous.column_type_family_as_enum(),
            differ.next.column_type_family_as_enum(),
        ) {
            (Some(previous_enum), Some(next_enum)) => {
                if all_match(previous_enum.values(), next_enum.values()) {
                    return None;
                }

                return Some(
                    if previous_enum
                        .values()
                        .all(|previous_value| next_enum.values().any(|next_value| previous_value == next_value))
                    {
                        ColumnTypeChange::SafeCast
                    } else {
                        ColumnTypeChange::RiskyCast
                    },
                );
            }
            (Some(_), None) | (None, Some(_)) => return Some(ColumnTypeChange::RiskyCast),
            (None, None) => (),
        }

        if differ.previous.column_type().native_type == differ.next.column_type().native_type
            && differ.previous.column_type().family == differ.next.column_type().family
        {
            return None;
        }

        match (
            differ.previous.column_native_type::<KingbaseMySqlType>(),
            differ.next.column_native_type::<KingbaseMySqlType>(),
        ) {
            (Some(previous), Some(next)) if datetime_precision_zero_is_unspecified(previous, next) => return None,
            (Some(previous), Some(next))
                if matches!(
                    (previous, next),
                    (KingbaseMySqlType::Int, KingbaseMySqlType::VarChar(_))
                ) =>
            {
                return Some(ColumnTypeChange::SafeCast);
            }
            // These conversions are delegated to the server. In particular an
            // unsupported BLOB-to-BIT change must fail, never drop/recreate data.
            (Some(previous), Some(next)) if native_conversion_attempt(previous, next) => {
                return Some(ColumnTypeChange::RiskyCast);
            }
            (Some(previous), Some(next)) if !explicit_cast_supported(Some(previous), Some(next)) => {
                return Some(ColumnTypeChange::NotCastable);
            }
            (None, Some(KingbaseMySqlType::TinyInt)) | (Some(KingbaseMySqlType::TinyInt), None)
                if differ.previous.column_type_family().is_boolean()
                    && differ.next.column_type_family().is_boolean() =>
            {
                return None;
            }
            _ => (),
        }

        Some(ColumnTypeChange::RiskyCast)
    }

    fn index_should_be_renamed(&self, indexes: MigrationPair<IndexWalker<'_>>) -> bool {
        indexes.previous.name() != indexes.next.name()
    }

    fn lower_cases_table_names(&self) -> bool {
        false
    }

    fn should_create_indexes_from_created_tables(&self) -> bool {
        // GIN and unique prefix indexes need separate CREATE INDEX steps.
        true
    }

    fn should_skip_index_for_new_table(&self, index: IndexWalker<'_>) -> bool {
        !super::index_is_created_separately(index)
    }

    fn should_ignore_json_defaults(&self) -> bool {
        true
    }

    fn should_recreate_fks_covered_by_deleted_indexes(&self) -> bool {
        true
    }

    fn table_names_match(&self, names: MigrationPair<&str>) -> bool {
        names.previous == names.next
    }
}

fn datetime_precision_zero_is_unspecified(previous: &KingbaseMySqlType, next: &KingbaseMySqlType) -> bool {
    matches!(
        (previous, next),
        (KingbaseMySqlType::Time(None), KingbaseMySqlType::Time(Some(0)))
            | (KingbaseMySqlType::Time(Some(0)), KingbaseMySqlType::Time(None))
            | (KingbaseMySqlType::DateTime(None), KingbaseMySqlType::DateTime(Some(0)))
            | (KingbaseMySqlType::DateTime(Some(0)), KingbaseMySqlType::DateTime(None))
            | (
                KingbaseMySqlType::Timestamp(None),
                KingbaseMySqlType::Timestamp(Some(0))
            )
            | (
                KingbaseMySqlType::Timestamp(Some(0)),
                KingbaseMySqlType::Timestamp(None)
            )
    )
}

fn is_blob(native_type: &KingbaseMySqlType) -> bool {
    matches!(
        native_type,
        KingbaseMySqlType::TinyBlob
            | KingbaseMySqlType::Blob
            | KingbaseMySqlType::MediumBlob
            | KingbaseMySqlType::LongBlob
    )
}

pub(super) fn is_binary(native_type: &KingbaseMySqlType) -> bool {
    matches!(
        native_type,
        KingbaseMySqlType::Bit(_) | KingbaseMySqlType::Binary(_) | KingbaseMySqlType::VarBinary(_)
    ) || is_blob(native_type)
}

fn is_bit(native_type: &KingbaseMySqlType) -> bool {
    matches!(native_type, KingbaseMySqlType::Bit(_))
}

pub(super) fn is_string(native_type: &KingbaseMySqlType) -> bool {
    matches!(
        native_type,
        KingbaseMySqlType::Char(_)
            | KingbaseMySqlType::VarChar(_)
            | KingbaseMySqlType::TinyText
            | KingbaseMySqlType::Text
            | KingbaseMySqlType::MediumText
            | KingbaseMySqlType::LongText
    )
}

pub(super) fn is_numeric(native_type: &KingbaseMySqlType) -> bool {
    matches!(
        native_type,
        KingbaseMySqlType::Int
            | KingbaseMySqlType::UnsignedInt
            | KingbaseMySqlType::SmallInt
            | KingbaseMySqlType::UnsignedSmallInt
            | KingbaseMySqlType::TinyInt
            | KingbaseMySqlType::UnsignedTinyInt
            | KingbaseMySqlType::MediumInt
            | KingbaseMySqlType::UnsignedMediumInt
            | KingbaseMySqlType::BigInt
            | KingbaseMySqlType::UnsignedBigInt
            | KingbaseMySqlType::Decimal(_)
            | KingbaseMySqlType::Float
            | KingbaseMySqlType::Double
            | KingbaseMySqlType::Year
    )
}

fn is_datetime(native_type: &KingbaseMySqlType) -> bool {
    matches!(
        native_type,
        KingbaseMySqlType::Date
            | KingbaseMySqlType::Time(_)
            | KingbaseMySqlType::DateTime(_)
            | KingbaseMySqlType::Timestamp(_)
    )
}

fn native_conversion_attempt(previous: &KingbaseMySqlType, next: &KingbaseMySqlType) -> bool {
    (is_blob(previous) && is_bit(next)) || (is_datetime(previous) && is_numeric(next))
}

pub(super) fn is_json(native_type: &KingbaseMySqlType) -> bool {
    matches!(native_type, KingbaseMySqlType::Json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_default_catalog_decoration_only_avoids_migrations_on_kingbase_numeric_columns() {
        use psl::parser_database::NoExtensionTypes;
        use schema_connector::SchemaDialect;

        for (provider, dialect) in [
            ("kingbase-mysql", crate::SqlSchemaDialect::kingbase_mysql()),
            ("mysql", crate::SqlSchemaDialect::mysql()),
            #[cfg(feature = "postgresql")]
            ("postgresql", crate::SqlSchemaDialect::postgres()),
            #[cfg(feature = "kingbase-oracle")]
            ("kingbase-oracle", crate::SqlSchemaDialect::kingbase_oracle()),
        ] {
            let calculate = |expression: &str, scalar: &str| {
                dialect.schema_from_datamodel(
                    vec![("schema.prisma".into(),format!(
                        "datasource db {{\n provider = \"{provider}\"\n }}\nmodel Product {{\n id Int @id\n price {scalar} @default(dbgenerated({}))\n }}",
                        serde_json::to_string(expression).unwrap()
                    ).into())],None,&NoExtensionTypes,
                ).unwrap()
            };
            let original = "1.25::numeric(10,1)";
            let decorated = "(1.25::numeric(3,2))::numeric(10,1)";
            for scalar in ["Decimal", "Float", "String"] {
                for (before, after) in [
                    (original, decorated),
                    (decorated, original),
                    ("(CAST('1.25' AS DECIMAL(10,1)))", decorated),
                    (decorated, "(CAST('1.25' AS DECIMAL(10,1)))"),
                ] {
                    let migration =
                        dialect.diff(calculate(before, scalar), calculate(after, scalar), &Default::default());
                    let sql = dialect.render_script(&migration, &Default::default()).unwrap();
                    if matches!(provider, "kingbase-mysql" | "kingbase-oracle") && scalar != "String" {
                        assert_eq!(sql, "-- This is an empty migration.");
                    } else {
                        assert_ne!(sql, "-- This is an empty migration.", "{provider}/{scalar}");
                    }
                }
            }
            for changed in [
                "1.25::numeric(10,2)",
                "1.35::numeric(10,1)",
                "(CAST('1.25' AS DECIMAL(10,2)))",
            ] {
                let migration = dialect.diff(
                    calculate(decorated, "Decimal"),
                    calculate(changed, "Decimal"),
                    &Default::default(),
                );
                let sql = dialect.render_script(&migration, &Default::default()).unwrap();
                assert_ne!(sql, "-- This is an empty migration.", "{provider}/{changed}");
                assert!(sql.contains(changed), "{provider}: {sql}");
            }
        }
    }

    #[test]
    fn widening_an_autoincrement_primary_key_preserves_the_constraint() {
        use psl::parser_database::NoExtensionTypes;
        use schema_connector::SchemaDialect;

        for provider in ["mysql", "kingbase-mysql"] {
            for next_default in ["@default(autoincrement())", ""] {
                for compound in [false, true] {
                    let dialect = if provider == "mysql" {
                        crate::SqlSchemaDialect::mysql()
                    } else {
                        crate::SqlSchemaDialect::kingbase_mysql()
                    };
                    let primary = if compound { "" } else { "@id" };
                    let model_key = if compound { "@@id([id, code])" } else { "" };
                    let calculate = |scalar: &str, default: &str| {
                        dialect
                            .schema_from_datamodel(
                                vec![(
                                    "schema.prisma".into(),
                                    format!(
                                        "datasource db {{\n provider = \"{provider}\"\n }}\n\
                                         model Item {{\n id {scalar} {primary} {default}\n code Int\n {model_key}\n }}"
                                    )
                                    .into(),
                                )],
                                None,
                                &NoExtensionTypes,
                            )
                            .unwrap()
                    };
                    let migration = dialect.diff(
                        calculate("Int", "@default(autoincrement())"),
                        calculate("BigInt", next_default),
                        &Default::default(),
                    );
                    let script = dialect.render_script(&migration, &Default::default()).unwrap();
                    let sequence = if next_default.is_empty() { "" } else { " AUTO_INCREMENT" };
                    assert!(
                        script.contains(&format!("MODIFY `id` BIGINT NOT NULL{sequence}")),
                        "{provider}/{compound}: {script}"
                    );
                    if provider == "kingbase-mysql" {
                        assert_eq!(
                            script,
                            format!("-- AlterTable\nALTER TABLE `Item` MODIFY `id` BIGINT NOT NULL{sequence};\n"),
                            "{compound}"
                        );
                    } else {
                        assert!(script.contains("DROP PRIMARY KEY"), "{script}");
                        assert!(script.contains("ADD PRIMARY KEY"), "{script}");
                    }
                }
            }
        }
    }

    #[test]
    fn other_primary_key_changes_still_recreate_the_constraint() {
        use psl::parser_database::NoExtensionTypes;
        use schema_connector::SchemaDialect;

        let dialect = crate::SqlSchemaDialect::kingbase_mysql();
        let calculate = |fields: &str| {
            dialect
                .schema_from_datamodel(
                    vec![(
                        "schema.prisma".into(),
                        format!(
                            "datasource db {{\n provider = \"kingbase-mysql\"\n }}\n\
                             model Item {{\n {fields}\n }}"
                        )
                        .into(),
                    )],
                    None,
                    &NoExtensionTypes,
                )
                .unwrap()
        };
        for (before, after) in [
            (
                "id Int @id @default(autoincrement())\n code Int",
                "id BigInt @unique @default(autoincrement())\n code Int @id",
            ),
            (
                "id Int @default(autoincrement())\n code Int\n @@id([id, code])",
                "id BigInt @default(autoincrement())\n code BigInt\n @@id([id, code])",
            ),
            (
                "id BigInt @id @default(autoincrement())",
                "id Int @id @default(autoincrement())",
            ),
            ("id Int @id", "id BigInt @id @default(autoincrement())"),
        ] {
            let migration = dialect.diff(calculate(before), calculate(after), &Default::default());
            let script = dialect.render_script(&migration, &Default::default()).unwrap();
            assert!(script.contains("DROP CONSTRAINT"), "{before} -> {after}: {script}");
            assert!(script.contains("ADD PRIMARY KEY"), "{before} -> {after}: {script}");
        }
    }

    fn temporal_decimal_migration(
        source: &str,
        target: &str,
        arity: &str,
        default: &str,
    ) -> schema_connector::Migration {
        use psl::parser_database::NoExtensionTypes;
        use schema_connector::SchemaDialect;
        let dialect = crate::SqlSchemaDialect::kingbase_mysql();
        let schema = |scalar: &str, native: &str, default: &str| {
            let datamodel = format!(
                "datasource db {{\n provider = \"kingbase-mysql\"\n }}\n \
                 model Event {{\n id Int @id\n value {scalar}{arity} @db.{native} {default}\n }}"
            );
            dialect
                .schema_from_datamodel(
                    vec![("schema.prisma".into(), datamodel.into())],
                    Some("public"),
                    &NoExtensionTypes,
                )
                .unwrap()
        };
        dialect.diff(
            schema("DateTime", source, ""),
            schema("Decimal", target, default),
            &Default::default(),
        )
    }

    #[test]
    fn temporal_numeric_changes_are_delegated_without_column_recreation() {
        use crate::sql_migration::{SqlMigration, SqlMigrationStep, TableChange};
        for (source, target) in [
            ("Date", "Decimal(10,2)"),
            ("Time(3)", "Decimal(8,2)"),
            ("DateTime(3)", "Decimal(16,2)"),
            ("Timestamp(3)", "Decimal(16,2)"),
            ("Date", "Decimal(8,2)"),
            ("Date", "Decimal(2,4)"),
        ] {
            let migration = temporal_decimal_migration(source, target, "?", "");
            let steps = &migration.downcast_ref::<SqlMigration>().steps;
            assert!(
                steps.iter().any(|step| match step {
                    SqlMigrationStep::AlterTable(table) => table
                        .changes
                        .iter()
                        .any(|change| matches!(change, TableChange::AlterColumn(_))),
                    _ => false,
                }),
                "{source} -> {target}"
            );
            let script = crate::apply_migration::render_script(
                &migration,
                &Default::default(),
                &super::super::renderer::KingbaseMysqlRenderer,
            )
            .unwrap();
            assert!(script.contains("USING (`value`::DECIMAL"), "{script}");
            assert!(
                !script.contains("to_char") && !script.contains("DROP COLUMN"),
                "{script}"
            );
        }
        assert!(native_conversion_attempt(
            &KingbaseMySqlType::Blob,
            &KingbaseMySqlType::Bit(8)
        ));
        assert!(!explicit_cast_supported(
            Some(&KingbaseMySqlType::Blob),
            Some(&KingbaseMySqlType::Bit(8))
        ));
    }

    #[test]
    fn explicit_using_support_is_based_on_the_source_and_target_types() {
        assert!(explicit_cast_supported(
            Some(&KingbaseMySqlType::DateTime(Some(0))),
            Some(&KingbaseMySqlType::Double),
        ));
        assert!(explicit_cast_supported(
            Some(&KingbaseMySqlType::VarChar(32)),
            Some(&KingbaseMySqlType::Json),
        ));
        assert!(explicit_cast_supported(
            Some(&KingbaseMySqlType::Json),
            Some(&KingbaseMySqlType::Blob),
        ));
        assert!(explicit_cast_supported(
            Some(&KingbaseMySqlType::Blob),
            Some(&KingbaseMySqlType::VarChar(32)),
        ));
    }

    #[test]
    fn unsupported_value_family_changes_stay_destructive() {
        assert!(!explicit_cast_supported(
            Some(&KingbaseMySqlType::Blob),
            Some(&KingbaseMySqlType::Int),
        ));
        assert!(!explicit_cast_supported(
            Some(&KingbaseMySqlType::Json),
            Some(&KingbaseMySqlType::Double),
        ));
        assert!(!explicit_cast_supported(
            Some(&KingbaseMySqlType::Int),
            Some(&KingbaseMySqlType::Date),
        ));
        assert!(!explicit_cast_supported(
            Some(&KingbaseMySqlType::Binary(8)),
            Some(&KingbaseMySqlType::Int),
        ));
        assert!(!explicit_cast_supported(
            Some(&KingbaseMySqlType::Json),
            Some(&KingbaseMySqlType::Bit(64)),
        ));
        assert!(explicit_cast_supported(
            Some(&KingbaseMySqlType::Date),
            Some(&KingbaseMySqlType::TinyInt),
        ));
        assert!(explicit_cast_supported(
            Some(&KingbaseMySqlType::Date),
            Some(&KingbaseMySqlType::Int),
        ));
    }
}

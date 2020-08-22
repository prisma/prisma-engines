use super::SqlSchemaDifferFlavour;
use crate::{flavour::PostgresFlavour, sql_migration::AlterEnum, sql_schema_differ::SqlSchemaDiffer};
use once_cell::sync::Lazy;
use regex::RegexSet;

impl SqlSchemaDifferFlavour for PostgresFlavour {
    fn alter_enums(&self, differ: &SqlSchemaDiffer<'_>) -> Vec<AlterEnum> {
        differ
            .enum_pairs()
            .filter_map(|differ| {
                let step = AlterEnum {
                    created_variants: differ.created_values().map(String::from).collect(),
                    dropped_variants: differ.dropped_values().map(String::from).collect(),
                    name: differ.previous.name.clone(),
                };

                if step.is_empty() {
                    None
                } else {
                    Some(step)
                }
            })
            .collect()
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

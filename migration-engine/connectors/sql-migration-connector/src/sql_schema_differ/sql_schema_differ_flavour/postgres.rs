use super::SqlSchemaDifferFlavour;
use crate::flavour::PostgresFlavour;
use once_cell::sync::Lazy;
use regex::RegexSet;

impl SqlSchemaDifferFlavour for PostgresFlavour {
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

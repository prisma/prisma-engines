use super::SqlSchemaDifferFlavour;
use crate::{
    flavour::SqliteFlavour, migration_pair::MigrationPair, sql_schema_differ::column::ColumnTypeChange,
    sql_schema_differ::differ_database::DifferDatabase,
};
use once_cell::sync::Lazy;
use regex::RegexSet;
use sql_schema_describer::{walkers::TableColumnWalker, ColumnTypeFamily};

/// These can be tables or views, depending on the PostGIS version. In both cases, they should be ignored.
static SPATIALITE_TABLES_OR_VIEWS: Lazy<RegexSet> = Lazy::new(|| {
    RegexSet::new([
        "(?i)^data_licenses$",
        "(?i)^elementarygeometries$",
        "(?i)^geometry_columns$",
        "(?i)^geometry_columns_auth$",
        "(?i)^geometry_columns_field_infos$",
        "(?i)^geometry_columns_statistics$",
        "(?i)^geometry_columns_time$",
        "(?i)^geom_cols_ref_sys$",
        "(?i)^idx_iso_metadata_geometry$",
        "(?i)^idx_iso_metadata_geometry_node$",
        "(?i)^idx_iso_metadata_geometry_parent$",
        "(?i)^idx_iso_metadata_geometry_rowid$",
        "(?i)^iso_metadata$",
        "(?i)^iso_metadata_reference$",
        "(?i)^iso_metadata_view$",
        "(?i)^knn$",
        "(?i)^knn2$",
        "(?i)^networks$",
        "(?i)^raster_coverages$",
        "(?i)^raster_coverages_keyword$",
        "(?i)^raster_coverages_ref_sys$",
        "(?i)^raster_coverages_srid$",
        "(?i)^rl2map_configurations$",
        "(?i)^rl2map_configurations_view$",
        "(?i)^se_external_graphics$",
        "(?i)^se_external_graphics_view$",
        "(?i)^se_fonts$",
        "(?i)^se_fonts_view$",
        "(?i)^se_raster_styled_layers$",
        "(?i)^se_raster_styled_layers_view$",
        "(?i)^se_raster_styles$",
        "(?i)^se_raster_styles_view$",
        "(?i)^se_vector_styled_layers$",
        "(?i)^se_vector_styled_layers_view$",
        "(?i)^se_vector_styles$",
        "(?i)^se_vector_styles_view$",
        "(?i)^spatialindex$",
        "(?i)^spatialite_history$",
        "(?i)^spatial_ref_sys$",
        "(?i)^spatial_ref_sys_all$",
        "(?i)^spatial_ref_sys_aux$",
        "(?i)^sql_statements_log$",
        "(?i)^stored_procedures$",
        "(?i)^stored_variables$",
        "(?i)^topologies$",
        "(?i)^vector_coverages$",
        "(?i)^vector_coverages_keyword$",
        "(?i)^vector_coverages_ref_sys$",
        "(?i)^vector_coverages_srid$",
        "(?i)^vector_layers$",
        "(?i)^vector_layers_auth$",
        "(?i)^vector_layers_field_infos$",
        "(?i)^vector_layers_statistics$",
        "(?i)^views_geometry_columns$",
        "(?i)^views_geometry_columns_auth$",
        "(?i)^views_geometry_columns_field_infos$",
        "(?i)^views_geometry_columns_statistics$",
        "(?i)^virts_geometry_collection$",
        "(?i)^virts_geometry_collectionm$",
        "(?i)^virts_geometry_columns$",
        "(?i)^virts_geometry_columns_auth$",
        "(?i)^virts_geometry_columns_field_infos$",
        "(?i)^virts_geometry_columns_statistics$",
        "(?i)^wms_getcapabilities$",
        "(?i)^wms_getmap$",
        "(?i)^wms_ref_sys$",
        "(?i)^wms_settings$",
    ])
    .unwrap()
});

impl SqlSchemaDifferFlavour for SqliteFlavour {
    fn can_rename_foreign_key(&self) -> bool {
        false
    }

    fn can_redefine_tables_with_inbound_foreign_keys(&self) -> bool {
        true
    }

    fn can_rename_index(&self) -> bool {
        false
    }

    fn column_autoincrement_changed(&self, _columns: MigrationPair<TableColumnWalker<'_>>) -> bool {
        false
    }

    fn column_type_change(&self, differ: MigrationPair<TableColumnWalker<'_>>) -> Option<ColumnTypeChange> {
        match (differ.previous.column_type_family(), differ.next.column_type_family()) {
            (a, b) if a == b => None,
            (_, ColumnTypeFamily::String) => Some(ColumnTypeChange::SafeCast),
            (_, _) => Some(ColumnTypeChange::RiskyCast),
        }
    }

    fn should_drop_indexes_from_dropped_tables(&self) -> bool {
        true
    }

    fn set_tables_to_redefine(&self, differ: &mut DifferDatabase<'_>) {
        differ.tables_to_redefine = differ
            .table_pairs()
            .filter(|differ| {
                differ.created_primary_key().is_some()
                    || differ.dropped_primary_key().is_some()
                    || differ.primary_key_changed()
                    || differ.dropped_columns().next().is_some()
                    || differ.added_columns().any(|col| col.arity().is_required())
                    || differ.any_column_changed()
                    || differ.created_foreign_keys().next().is_some()
                    || differ.dropped_foreign_keys().next().is_some()
            })
            .map(|table| table.table_ids())
            .collect();
    }

    fn should_drop_foreign_keys_from_dropped_tables(&self) -> bool {
        false
    }

    fn should_push_foreign_keys_from_created_tables(&self) -> bool {
        false
    }

    fn has_unnamed_foreign_keys(&self) -> bool {
        true
    }

    fn table_should_be_ignored(&self, table_name: &str) -> bool {
        SPATIALITE_TABLES_OR_VIEWS.is_match(table_name)
    }

    fn view_should_be_ignored(&self, view_name: &str) -> bool {
        SPATIALITE_TABLES_OR_VIEWS.is_match(view_name)
    }
}

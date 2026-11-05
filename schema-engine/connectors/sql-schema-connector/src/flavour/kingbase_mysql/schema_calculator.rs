use psl::{datamodel_connector::constraint_names::ConstraintNames, parser_database::walkers::EnumWalker};

use sql_schema_describer as sql;

use crate::sql_schema_calculator::{Context, SqlSchemaCalculatorFlavour};

/// Kingbase MySQL has MySQL-compatible enum storage, but its native type mapping
/// must be calculated through the Kingbase PSL connector.
#[derive(Debug, Default)]
pub(crate) struct KingbaseMysqlSchemaCalculatorFlavour;

impl SqlSchemaCalculatorFlavour for KingbaseMysqlSchemaCalculatorFlavour {
    fn datamodel_connector(&self) -> &dyn psl::datamodel_connector::Connector {
        psl::builtin_connectors::KINGBASE_MYSQL
    }

    fn calculate_enums(&self, ctx: &mut Context<'_>) {
        let enum_fields = ctx
            .datamodel
            .db
            .walk_models()
            .flat_map(|model| model.scalar_fields())
            .filter_map(|field| field.field_type_as_enum().map(|enum_walker| (field, enum_walker)));

        for (field, enum_tpe) in enum_fields {
            let name = format!(
                "{model_name}_{field_name}",
                model_name = field.model().database_name(),
                field_name = field.database_name()
            );
            let sql_enum_id = ctx.schema.describer_schema.push_enum(Default::default(), name, None);
            ctx.enum_ids.insert(enum_tpe.id, sql_enum_id);
            for variant in enum_tpe.values().map(|v| v.database_name().to_owned()) {
                ctx.schema.describer_schema.push_enum_variant(sql_enum_id, variant);
            }
        }
    }

    fn column_type_for_enum(&self, enm: EnumWalker<'_>, ctx: &Context<'_>) -> Option<sql::ColumnTypeFamily> {
        ctx.enum_ids.get(&enm.id).map(|id| sql::ColumnTypeFamily::Enum(*id))
    }

    fn push_connector_data(&self, context: &mut Context<'_>) {
        let schema = &mut context.schema.describer_schema;
        // Only inline unique indexes create constraints. Descending unique
        // indexes are created separately; described databases use sys_constraint.
        let physical_index_names: std::collections::HashMap<_, _> = schema
            .table_walkers()
            .flat_map(|table| table.indexes())
            .filter(|index| !index.is_primary_key())
            .map(|index| {
                (
                    index.id,
                    sql::kingbase_mysql::physical_index_name(index.table().name(), index.name()),
                )
            })
            .collect();
        let unique_constraint_names = schema
            .table_walkers()
            .flat_map(|table| table.indexes())
            .filter(|index| index.is_unique() && !super::index_is_created_separately(*index))
            .map(|index| {
                (
                    index.id,
                    physical_index_names
                        .get(&index.id)
                        .cloned()
                        .unwrap_or_else(|| index.name().to_owned()),
                )
            })
            .collect();
        // A calculated schema represents fresh CREATE TABLE statements, so
        // its unnamed primary keys use the default name with the connector's
        // identifier limit. Described schemas always retain the catalog name.
        let primary_key_constraint_names = schema
            .table_walkers()
            .filter_map(|table| {
                table.primary_key().map(|primary_key| {
                    let name = if primary_key.name().is_empty() {
                        ConstraintNames::primary_key_name(table.name(), self.datamodel_connector())
                    } else {
                        primary_key.name().to_owned()
                    };
                    (primary_key.id, name)
                })
            })
            .collect();

        schema.set_connector_data(Box::new(sql::kingbase_mysql::KingbaseMysqlSchemaExt {
            physical_index_names,
            unique_constraint_names,
            primary_key_constraint_names,
        }));
    }
}

use super::{super::Context, SqlSchemaCalculatorFlavour};
use crate::flavour::MysqlFlavour;

impl SqlSchemaCalculatorFlavour for MysqlFlavour {
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
}

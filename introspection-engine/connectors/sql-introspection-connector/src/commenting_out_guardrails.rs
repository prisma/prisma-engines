use crate::calculate_datamodel::CalculateDatamodelContext;
use crate::warnings::{
    warning_enum_values_with_empty_names, warning_fields_with_empty_names, warning_models_without_columns,
    warning_models_without_identifier, warning_unsupported_types, EnumAndValue, Model, ModelAndField,
    ModelAndFieldAndType,
};
use crate::SqlFamilyTrait;
use introspection_connector::Warning;
use psl::dml::{Datamodel, FieldType};

pub(crate) fn commenting_out_guardrails(datamodel: &mut Datamodel, ctx: &CalculateDatamodelContext) -> Vec<Warning> {
    let mut warnings = vec![];

    // order matters...
    let models_without_columns = models_without_columns(datamodel, ctx);
    let models_without_identifiers = models_wihtout_uniques(datamodel, &models_without_columns);
    let fields_with_empty_names = fields_with_empty_names(datamodel);
    let enum_values_with_empty_names = empty_enum_values(datamodel);
    let unsupported_types = unsupported_types(datamodel);

    if !models_without_columns.is_empty() {
        warnings.push(warning_models_without_columns(&models_without_columns))
    }

    if !models_without_identifiers.is_empty() {
        warnings.push(warning_models_without_identifier(&models_without_identifiers))
    }

    if !fields_with_empty_names.is_empty() {
        warnings.push(warning_fields_with_empty_names(&fields_with_empty_names))
    }

    if !unsupported_types.is_empty() {
        warnings.push(warning_unsupported_types(&unsupported_types))
    }

    if !enum_values_with_empty_names.is_empty() {
        warnings.push(warning_enum_values_with_empty_names(&enum_values_with_empty_names))
    }

    warnings
}

// on postgres this is allowed, on the other dbs, this could be a symptom of
// missing privileges
fn models_without_columns(datamodel: &mut Datamodel, ctx: &CalculateDatamodelContext) -> Vec<Model> {
    let mut models_without_columns = vec![];

    for model in datamodel.models_mut() {
        if model.fields.is_empty() {
            model.is_commented_out = true;
            let comment = match ctx.sql_family().is_postgres() {
                true =>
                    "We could not retrieve columns for the underlying table. Either it has none or you are missing rights to see them. Please check your privileges.".to_string(),
               false=> "We could not retrieve columns for the underlying table. You probably have no rights to see them. Please check your privileges.".to_string(),

            };
            //postgres could be valid, or privileges, commenting out because we cannot handle it.
            //others, this is invalid, commenting out because we cannot handle it.
            model.documentation = Some(comment);
            models_without_columns.push(Model {
                model: model.name.clone(),
            })
        }
    }

    models_without_columns
}

// models without uniques / ids
fn models_wihtout_uniques(datamodel: &mut Datamodel, models_without_columns: &[Model]) -> Vec<Model> {
    let mut models_without_identifiers = vec![];

    for model in datamodel
        .models_mut()
        .filter(|model| !models_without_columns.iter().any(|m| m.model == model.name))
    {
        if model.strict_unique_criterias_disregarding_unsupported().is_empty() {
            model.is_ignored = true;
            model.documentation = Some(
                "The underlying table does not contain a valid unique identifier and can therefore currently not be handled by the Prisma Client."
                    .to_string(),
            );
            models_without_identifiers.push(Model {
                model: model.name.clone(),
            })
        }
    }

    // remove their backrelations
    for model_without_identifier in &models_without_identifiers {
        for model in datamodel.models_mut() {
            let model_is_ignored = model.is_ignored;
            for field in model.relation_fields_mut() {
                if field.points_to_model(&model_without_identifier.model) && !model_is_ignored {
                    field.is_ignored = true;
                }
            }
        }
    }

    models_without_identifiers
}

fn fields_with_empty_names(datamodel: &mut Datamodel) -> Vec<ModelAndField> {
    let mut fields_with_empty_names = vec![];

    for model in datamodel.models_mut() {
        let model_name = model.name.clone();

        for field in model.scalar_fields_mut() {
            if field.name.is_empty() {
                field.documentation = Some(
                    "This field was commented out because of an invalid name. Please provide a valid one that matches [a-zA-Z][a-zA-Z0-9_]*"
                        .to_string(),
                );
                field.name = field.database_name.as_ref().unwrap().to_string();
                field.is_commented_out = true;

                fields_with_empty_names.push(ModelAndField::new(&model_name, &field.name))
            }
        }
    }

    fields_with_empty_names
}

fn empty_enum_values(datamodel: &mut Datamodel) -> Vec<EnumAndValue> {
    let mut enum_values_with_empty_names = vec![];

    for enm in datamodel.enums_mut() {
        let enum_name = enm.name.clone();

        for enum_value in enm.values_mut() {
            let name = match &enum_value.database_name {
                Some(name) => name,
                None => continue,
            };

            if !enum_value.name.is_empty() {
                continue;
            }

            enum_value.name = name.clone();
            enum_value.commented_out = true;
            enum_values_with_empty_names.push(EnumAndValue::new(&enum_name, &enum_value.name))
        }
    }

    enum_values_with_empty_names
}

// fields with unsupported as datatype
fn unsupported_types(datamodel: &mut Datamodel) -> Vec<ModelAndFieldAndType> {
    let mut unsupported_types = vec![];

    for model in datamodel.models_mut() {
        let model_name = model.name.clone();

        for field in model.scalar_fields_mut() {
            let r#type = match &field.field_type {
                FieldType::Unsupported(r#type) => r#type,
                _ => continue,
            };

            unsupported_types.push(ModelAndFieldAndType {
                model: model_name.clone(),
                field: field.name.clone(),
                tpe: r#type.clone(),
            })
        }
    }

    unsupported_types
}

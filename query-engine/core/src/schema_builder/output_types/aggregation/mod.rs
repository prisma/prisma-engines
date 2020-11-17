use super::output_objects::map_scalar_output_type;
use super::*;
use prisma_models::ScalarFieldRef;

pub(crate) mod group_by;
pub(crate) mod plain;

fn field_avg_output_type(field: &ScalarFieldRef) -> OutputType {
    match field.type_identifier {
        TypeIdentifier::Int | TypeIdentifier::BigInt | TypeIdentifier::Float => OutputType::float(),
        TypeIdentifier::Decimal => OutputType::decimal(),
        _ => map_scalar_output_type(field),
    }
}

fn collect_non_list_fields(model: &ModelRef) -> Vec<ScalarFieldRef> {
    model.fields().scalar().into_iter().filter(|f| !f.is_list).collect()
}

fn collect_numeric_fields(model: &ModelRef) -> Vec<ScalarFieldRef> {
    model
        .fields()
        .scalar()
        .into_iter()
        .filter(|field| is_numeric(field))
        .collect()
}

fn is_numeric(field: &ScalarFieldRef) -> bool {
    matches!(
        field.type_identifier,
        TypeIdentifier::Int | TypeIdentifier::BigInt | TypeIdentifier::Float | TypeIdentifier::Decimal
    )
}

use super::*;

pub fn get_field_filter_type<'a>(field: &ModelField) -> InputObjectTypeWeakRef {
    match field {
        ModelField::Relation(rf) => relation_filter_type(rf),
        ModelField::Scalar(sf) if field.is_list() => scalar_list_filter_type(sf),
        ModelField::Scalar(sf) => scalar_filter_type(sf),
    }
}

fn relation_filter_type(rf: &RelationFieldRef) -> InputObjectTypeWeakRef {
    todo!()

    // ModelField::Relation(_) if field.is_list() => vec![&args.multi_relation],
    // ModelField::Relation(_) => vec![&args.one_relation],
}

fn scalar_list_filter_type(sf: &ScalarFieldRef) -> InputObjectTypeWeakRef {
    todo!()
}

fn scalar_filter_type(sf: &ScalarFieldRef) -> InputObjectTypeWeakRef {
    //   match sf.type_identifier {
    //     TypeIdentifier::UUID => vec![&args.base, &args.inclusion, &args.alphanumeric, &args.string],
    //     TypeIdentifier::String => vec![&args.base, &args.inclusion, &args.alphanumeric, &args.string],
    //     TypeIdentifier::Int => vec![&args.base, &args.inclusion, &args.alphanumeric],
    //     TypeIdentifier::Float => vec![&args.base, &args.inclusion, &args.alphanumeric],
    //     TypeIdentifier::Boolean => vec![&args.base],
    //     TypeIdentifier::Enum(_) => vec![&args.base, &args.inclusion],
    //     TypeIdentifier::DateTime => vec![&args.base, &args.inclusion, &args.alphanumeric],
    //     TypeIdentifier::Json => vec![&args.base],
    // },

    todo!()
}

pub(crate) fn string_filter_fields() -> InputObjectTypeWeakRef {
    todo!()
}

pub fn equality_filters() -> impl Iterator<Item = InputField> {
    todo!()
}

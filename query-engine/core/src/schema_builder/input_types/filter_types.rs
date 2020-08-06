use super::*;

pub(crate) fn get_field_filter_type(ctx: &mut BuilderContext, field: &ModelField) -> InputObjectTypeWeakRef {
    match field {
        ModelField::Relation(rf) => relation_filter_type(rf),
        ModelField::Scalar(sf) if field.is_list() => scalar_list_filter_type(ctx, sf),
        ModelField::Scalar(sf) => scalar_filter_type(sf),
    }
}

fn relation_filter_type(rf: &RelationFieldRef) -> InputObjectTypeWeakRef {
    if rf.is_list {
        // ModelField::Relation(_)  => vec![&args.multi_relation],
    } else {
        // ModelField::Relation(_) => vec![&args.one_relation],
    }

    todo!()
}

fn scalar_list_filter_type(ctx: &mut BuilderContext, sf: &ScalarFieldRef) -> InputObjectTypeWeakRef {
    let name = filter_name(&sf.type_identifier);
    return_cached_input!(ctx, &name);

    let obj = Arc::new(init_input_object_type(name.clone()));
    ctx.cache_input_type(name, obj.clone());

    let fields = equality_filters(sf);
    obj.set_fields(fields);

    Arc::downgrade(&obj)
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

pub fn equality_filters(sf: &ScalarFieldRef) -> Vec<InputField> {
    vec![
        input_field("equal", map_required_input_type(sf), None),
        input_field("not_equal", map_required_input_type(sf), None),
    ]
}

fn filter_name(ident: &TypeIdentifier) -> String {
    match ident {
        TypeIdentifier::UUID => "UuidFilter".to_owned(),
        TypeIdentifier::String => "StringFilter".to_owned(),
        TypeIdentifier::Int => "IntFilter".to_owned(),
        TypeIdentifier::Float => "FloatFilter".to_owned(),
        TypeIdentifier::Boolean => "BoolFilter".to_owned(),
        TypeIdentifier::Enum(e) => format!("Enum{}Filter", e),
        TypeIdentifier::DateTime => "DateTimeFilter".to_owned(),
        TypeIdentifier::Json => "JsonFilter".to_owned(),
    }
}

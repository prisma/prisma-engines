use super::*;

pub(crate) fn get_field_filter_type(ctx: &mut BuilderContext, field: &ModelField) -> InputObjectTypeWeakRef {
    match field {
        ModelField::Relation(rf) => relation_filter_type(ctx, rf),
        ModelField::Scalar(sf) if field.is_list() => scalar_list_filter_type(ctx, sf),
        ModelField::Scalar(sf) => scalar_filter_type(ctx, sf),
    }
}

fn relation_filter_type(ctx: &mut BuilderContext, rf: &RelationFieldRef) -> InputObjectTypeWeakRef {
    let related_model = rf.related_model();
    let related_input_type = filter_input_objects::where_object_type(ctx, &related_model);
    let filter_name = format!("{}RelationFilter", capitalize(&related_model.name));

    return_cached_input!(ctx, &filter_name);
    let object = Arc::new(init_input_object_type(filter_name.clone()));
    ctx.cache_input_type(filter_name, object.clone());

    let fields = if rf.is_list {
        vec![
            input_field("every", wrap_opt_input_object(related_input_type.clone()), None),
            input_field("some", wrap_opt_input_object(related_input_type.clone()), None),
            input_field("none", wrap_opt_input_object(related_input_type.clone()), None),
        ]
    } else {
        vec![input_field(
            "is",
            InputType::opt(InputType::null(InputType::object(related_input_type))),
            None,
        )]
    };

    object.set_fields(fields);
    Arc::downgrade(&object)
}

fn map_relation_filter_input_field(ctx: &mut BuilderContext, field: &RelationFieldRef) -> Vec<InputField> {
    let related_model = field.related_model();
    let related_input_type = filter_input_objects::where_object_type(ctx, &related_model);

    let input_fields: Vec<_> = filter_arguments::get_field_filters(&ModelField::Relation(field.clone()))
        .into_iter()
        .map(|arg| {
            let field_name = format!("{}{}", field.name, arg.suffix);
            let obj = InputType::object(related_input_type.clone());
            let typ = if arg.suffix == "" { InputType::null(obj) } else { obj }; // ONLY one relation

            input_field(field_name, InputType::opt(typ), None)
        })
        .collect();

    input_fields
}

fn scalar_list_filter_type(ctx: &mut BuilderContext, sf: &ScalarFieldRef) -> InputObjectTypeWeakRef {
    let name = scalar_filter_name(&sf.type_identifier);
    return_cached_input!(ctx, &name);

    let object = Arc::new(init_input_object_type(name.clone()));
    ctx.cache_input_type(name, object.clone());

    let fields = equality_filters(sf);
    object.set_fields(fields);

    Arc::downgrade(&object)
}

fn scalar_filter_type(ctx: &mut BuilderContext, sf: &ScalarFieldRef) -> InputObjectTypeWeakRef {
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

fn scalar_filter_name(ident: &TypeIdentifier) -> String {
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

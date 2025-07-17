use super::*;
use constants::*;
use query_structure::CompositeFieldRef;

pub(crate) struct UpdateDataInputFieldMapper {
    unchecked: bool,
}

impl UpdateDataInputFieldMapper {
    pub fn new_checked() -> Self {
        Self { unchecked: false }
    }

    pub fn new_unchecked() -> Self {
        Self { unchecked: true }
    }
}

impl DataInputFieldMapper for UpdateDataInputFieldMapper {
    fn map_scalar<'a>(&self, ctx: &'a QuerySchema, sf: ScalarFieldRef) -> InputField<'a> {
        let base_update_type = match sf.type_identifier() {
            TypeIdentifier::Float => InputType::object(update_operations_object_type(ctx, "Float", sf.clone(), true)),
            TypeIdentifier::Decimal => {
                InputType::object(update_operations_object_type(ctx, "Decimal", sf.clone(), true))
            }
            TypeIdentifier::Int => InputType::object(update_operations_object_type(ctx, "Int", sf.clone(), true)),
            TypeIdentifier::BigInt => InputType::object(update_operations_object_type(ctx, "BigInt", sf.clone(), true)),
            TypeIdentifier::String => {
                InputType::object(update_operations_object_type(ctx, "String", sf.clone(), false))
            }
            TypeIdentifier::Boolean => InputType::object(update_operations_object_type(ctx, "Bool", sf.clone(), false)),
            TypeIdentifier::Enum(enum_id) => {
                let enum_name = ctx.internal_data_model.walk(enum_id).name();
                InputType::object(update_operations_object_type(
                    ctx,
                    &format!("Enum{enum_name}"),
                    sf.clone(),
                    false,
                ))
            }
            TypeIdentifier::Json => map_scalar_input_type_for_field(ctx, &sf),
            TypeIdentifier::DateTime => {
                InputType::object(update_operations_object_type(ctx, "DateTime", sf.clone(), false))
            }
            TypeIdentifier::UUID => InputType::object(update_operations_object_type(ctx, "Uuid", sf.clone(), false)),
            TypeIdentifier::Bytes => InputType::object(update_operations_object_type(ctx, "Bytes", sf.clone(), false)),

            TypeIdentifier::Unsupported => unreachable!("No unsupported field should reach that path"),
        };

        let has_adv_json = ctx.has_capability(ConnectorCapability::AdvancedJsonNullability);
        match sf.type_identifier() {
            TypeIdentifier::Json if has_adv_json => {
                let enum_type = InputType::enum_type(json_null_input_enum(!sf.is_required()));
                let input_field = input_field(sf.name().to_owned(), vec![enum_type, base_update_type], None);

                input_field.optional()
            }

            _ => {
                let types = vec![map_scalar_input_type_for_field(ctx, &sf), base_update_type];

                let input_field = input_field(sf.name().to_owned(), types, None);
                input_field.optional().nullable_if(!sf.is_required())
            }
        }
    }

    fn map_scalar_list<'a>(&self, ctx: &'a QuerySchema, sf: ScalarFieldRef) -> InputField<'a> {
        let list_input_type = map_scalar_input_type(ctx, sf.type_identifier(), sf.is_list());
        let cloned_list_input_type = list_input_type.clone();
        let ident = Identifier::new_prisma(IdentifierType::ScalarListUpdateInput(sf.clone()));
        let type_identifier = sf.type_identifier();

        let mut input_object = init_input_object_type(ident);
        input_object.set_container(sf.container());
        input_object.set_fields(move || {
            let mut object_fields = vec![simple_input_field(operations::SET, list_input_type.clone(), None).optional()];

            if ctx.has_capability(ConnectorCapability::ScalarLists)
                && (ctx.has_capability(ConnectorCapability::EnumArrayPush) || !type_identifier.is_enum())
            {
                let map_scalar_type = map_scalar_input_type(ctx, type_identifier, false);
                object_fields.push(
                    input_field(operations::PUSH, vec![map_scalar_type, list_input_type.clone()], None).optional(),
                )
            }

            object_fields
        });
        input_object.require_exactly_one_field();

        let input_type = InputType::object(input_object);
        input_field(sf.name().to_owned(), vec![input_type, cloned_list_input_type], None).optional()
    }

    fn map_relation<'a>(&self, ctx: &'a QuerySchema, rf: RelationFieldRef) -> InputField<'a> {
        let ident = Identifier::new_prisma(IdentifierType::RelationUpdateInput(
            rf.clone(),
            rf.related_field(),
            self.unchecked,
        ));
        let rf_name = rf.name().to_owned();

        let mut input_object = init_input_object_type(ident);
        input_object.set_container(rf.related_model());
        input_object.set_fields(move || {
            let mut fields = vec![];

            if rf.related_model().supports_create_operation() {
                fields.push(input_fields::nested_create_one_input_field(ctx, rf.clone()));

                append_opt(
                    &mut fields,
                    input_fields::nested_connect_or_create_field(ctx, rf.clone()),
                );
                append_opt(&mut fields, input_fields::nested_upsert_field(ctx, rf.clone()));
                append_opt(
                    &mut fields,
                    input_fields::nested_create_many_input_field(ctx, rf.clone()),
                );
            }

            append_opt(&mut fields, input_fields::nested_set_input_field(ctx, &rf));
            append_opt(&mut fields, input_fields::nested_disconnect_input_field(ctx, &rf));
            append_opt(&mut fields, input_fields::nested_delete_input_field(ctx, &rf));

            fields.push(input_fields::nested_connect_input_field(ctx, &rf));
            fields.push(input_fields::nested_update_input_field(ctx, rf.clone()));

            append_opt(&mut fields, input_fields::nested_update_many_field(ctx, rf.clone()));
            append_opt(&mut fields, input_fields::nested_delete_many_field(ctx, &rf));
            fields
        });

        simple_input_field(rf_name, InputType::object(input_object), None).optional()
    }

    fn map_composite<'a>(&self, ctx: &'a QuerySchema, cf: CompositeFieldRef) -> InputField<'a> {
        // Shorthand object (equivalent to the "set" operation).
        let shorthand_type = InputType::Object(create::composite_create_object_type(ctx, cf.clone()));

        // Operation envelope object.
        let envelope_type = InputType::Object(composite_update_envelope_object_type(ctx, cf.clone()));

        let mut input_types = vec![envelope_type, shorthand_type.clone()];

        if cf.is_list() {
            input_types.push(InputType::list(shorthand_type));
        }

        input_field(cf.name().to_owned(), input_types, None)
            .nullable_if(cf.is_optional() && !cf.is_list())
            .optional()
    }
}

fn update_operations_object_type<'a>(
    ctx: &'a QuerySchema,
    prefix: &str,
    sf: ScalarField,
    with_number_operators: bool,
) -> InputObjectType<'a> {
    let ident = Identifier::new_prisma(IdentifierType::FieldUpdateOperationsInput(
        !sf.is_required(),
        prefix.to_owned(),
    ));

    let mut obj = init_input_object_type(ident);
    obj.set_container(sf.container());
    obj.require_exactly_one_field();
    obj.set_fields(move || {
        let typ = map_scalar_input_type_for_field(ctx, &sf);
        let mut fields = vec![
            simple_input_field(operations::SET, typ.clone(), None)
                .optional()
                .nullable_if(!sf.is_required()),
        ];

        if with_number_operators {
            fields.push(simple_input_field(operations::INCREMENT, typ.clone(), None).optional());
            fields.push(simple_input_field(operations::DECREMENT, typ.clone(), None).optional());
            fields.push(simple_input_field(operations::MULTIPLY, typ.clone(), None).optional());
            fields.push(simple_input_field(operations::DIVIDE, typ, None).optional());
        }

        if ctx.has_capability(ConnectorCapability::UndefinedType) && !sf.is_required() {
            fields.push(simple_input_field(operations::UNSET, InputType::boolean(), None).optional());
        }

        fields
    });
    obj
}

/// Build an operation envelope object type for composite updates.
/// An operation envelope is an object that encapsulates the possible operations, like:
/// ```text
/// cf_field: { // this is the envelope object
///   set:    { ... set type ... }
///   update: { ... update type ... }
///   ... more ops ...
/// }
/// ```
fn composite_update_envelope_object_type(ctx: &'_ QuerySchema, cf: CompositeFieldRef) -> InputObjectType<'_> {
    let ident = Identifier::new_prisma(IdentifierType::CompositeUpdateEnvelopeInput(cf.typ(), cf.arity()));

    let mut input_object = init_input_object_type(ident);
    input_object.set_container(cf.typ());
    input_object.require_exactly_one_field();
    input_object.set_tag(ObjectTag::CompositeEnvelope);
    input_object.set_fields(move || {
        let mut fields = vec![composite_set_update_input_field(ctx, &cf)];

        append_opt(&mut fields, composite_update_input_field(ctx, cf.clone()));
        append_opt(&mut fields, composite_push_update_input_field(ctx, &cf));
        append_opt(&mut fields, composite_upsert_update_input_field(ctx, cf.clone()));
        append_opt(&mut fields, composite_update_many_update_input_field(ctx, cf.clone()));
        append_opt(&mut fields, composite_delete_many_update_input_field(ctx, cf.clone()));
        append_opt(&mut fields, composite_unset_update_input_field(&cf));

        fields
    });
    input_object
}

/// Builds the `update` input object type. Should be used in the envelope type.
fn composite_update_object_type(ctx: &'_ QuerySchema, cf: CompositeFieldRef) -> InputObjectType<'_> {
    let ident = Identifier::new_prisma(IdentifierType::CompositeUpdateInput(cf.typ()));

    let mut input_object = init_input_object_type(ident);
    input_object.set_container(cf.typ());
    input_object.set_min_fields(1);
    input_object.set_fields(move || {
        let mapper = UpdateDataInputFieldMapper::new_checked();
        let typ = cf.typ();
        let mut fields = typ.fields();
        mapper.map_all(ctx, &mut fields)
    });
    input_object
}

// Builds an `update` input field. Should only be used in the envelope type.
fn composite_update_input_field(ctx: &'_ QuerySchema, cf: CompositeFieldRef) -> Option<InputField<'_>> {
    if cf.is_required() {
        let update_object_type = composite_update_object_type(ctx, cf);

        Some(simple_input_field(operations::UPDATE, InputType::Object(update_object_type), None).optional())
    } else {
        None
    }
}

// Builds an `unset` input field. Should only be used in the envelope type.
fn composite_unset_update_input_field<'a>(cf: &CompositeFieldRef) -> Option<InputField<'a>> {
    if cf.is_optional() {
        Some(simple_input_field(operations::UNSET, InputType::boolean(), None).optional())
    } else {
        None
    }
}

// Builds an `set` input field. Should only be used in the envelope type.
fn composite_set_update_input_field<'a>(ctx: &'a QuerySchema, cf: &CompositeFieldRef) -> InputField<'a> {
    let set_object_type = InputType::Object(create::composite_create_object_type(ctx, cf.clone()));

    let mut input_types = vec![set_object_type.clone()];

    if cf.is_list() {
        input_types.push(InputType::list(set_object_type));
    }

    input_field(operations::SET, input_types, None)
        .nullable_if(!cf.is_required() && !cf.is_list())
        .optional()
}

// Builds an `push` input field. Should only be used in the envelope type.
fn composite_push_update_input_field<'a>(ctx: &'a QuerySchema, cf: &CompositeFieldRef) -> Option<InputField<'a>> {
    if cf.is_list() {
        let set_object_type = InputType::Object(create::composite_create_object_type(ctx, cf.clone()));
        let input_types = vec![set_object_type.clone(), InputType::list(set_object_type)];

        Some(input_field(operations::PUSH, input_types, None).optional())
    } else {
        None
    }
}

/// Builds the `upsert` input object type. Should only be used in the envelope type.
fn composite_upsert_object_type(ctx: &'_ QuerySchema, cf: CompositeFieldRef) -> InputObjectType<'_> {
    let ident = Identifier::new_prisma(IdentifierType::CompositeUpsertObjectInput(cf.typ()));

    let mut input_object = init_input_object_type(ident);
    input_object.set_container(cf.typ());
    input_object.set_tag(ObjectTag::CompositeEnvelope);
    input_object.set_fields(move || {
        let update_object_type = composite_update_object_type(ctx, cf.clone());
        let update_field = simple_input_field(operations::UPDATE, InputType::Object(update_object_type), None);
        let set_field = composite_set_update_input_field(ctx, &cf).required();

        vec![set_field, update_field]
    });
    input_object
}

// Builds an `upsert` input field. Should only be used in the envelope type.
fn composite_upsert_update_input_field(ctx: &'_ QuerySchema, cf: CompositeFieldRef) -> Option<InputField<'_>> {
    if cf.is_optional() {
        let upsert_object_type = InputType::Object(composite_upsert_object_type(ctx, cf));

        Some(simple_input_field(operations::UPSERT, upsert_object_type, None).optional())
    } else {
        None
    }
}

fn composite_update_many_object_type(ctx: &'_ QuerySchema, cf: CompositeFieldRef) -> InputObjectType<'_> {
    let ident = Identifier::new_prisma(IdentifierType::CompositeUpdateManyInput(cf.typ()));

    let mut input_object = init_input_object_type(ident);
    input_object.set_container(cf.typ());
    input_object.set_tag(ObjectTag::CompositeEnvelope);
    input_object.set_fields(move || {
        let where_object_type = objects::filter_objects::where_object_type(ctx, cf.typ().into());
        let where_field = simple_input_field(args::WHERE, InputType::object(where_object_type), None);

        let update_object_type = composite_update_object_type(ctx, cf);
        let data_field = simple_input_field(args::DATA, InputType::Object(update_object_type), None);

        vec![where_field, data_field]
    });

    input_object
}

fn composite_delete_many_object_type(ctx: &'_ QuerySchema, cf: CompositeFieldRef) -> InputObjectType<'_> {
    let ident = Identifier::new_prisma(IdentifierType::CompositeDeleteManyInput(cf.typ()));

    let mut input_object = init_input_object_type(ident);
    input_object.set_container(cf.typ());
    input_object.set_tag(ObjectTag::CompositeEnvelope);
    input_object.set_fields(move || {
        let where_object_type = objects::filter_objects::where_object_type(ctx, cf.typ().into());
        let where_field = simple_input_field(args::WHERE, InputType::object(where_object_type), None);

        vec![where_field]
    });
    input_object
}

// Builds an `updateMany` input field. Should only be used in the envelope type.
fn composite_update_many_update_input_field(ctx: &'_ QuerySchema, cf: CompositeFieldRef) -> Option<InputField<'_>> {
    if cf.is_list() {
        let update_many = InputType::Object(composite_update_many_object_type(ctx, cf));

        Some(simple_input_field(operations::UPDATE_MANY, update_many, None).optional())
    } else {
        None
    }
}

// Builds a `deleteMany` input field. Should only be used in the envelope type.
fn composite_delete_many_update_input_field(ctx: &'_ QuerySchema, cf: CompositeFieldRef) -> Option<InputField<'_>> {
    if cf.is_list() {
        let delete_many = InputType::Object(composite_delete_many_object_type(ctx, cf));

        Some(simple_input_field(operations::DELETE_MANY, delete_many, None).optional())
    } else {
        None
    }
}

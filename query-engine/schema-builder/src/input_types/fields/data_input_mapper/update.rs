use super::*;
use crate::{constants::*, enum_types::*};
use prisma_models::CompositeFieldRef;

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
    fn map_scalar(&self, ctx: &mut BuilderContext, sf: &ScalarFieldRef) -> InputField {
        let base_update_type = match sf.type_identifier() {
            TypeIdentifier::Float => InputType::object(update_operations_object_type(ctx, "Float", sf, true)),
            TypeIdentifier::Decimal => InputType::object(update_operations_object_type(ctx, "Decimal", sf, true)),
            TypeIdentifier::Int => InputType::object(update_operations_object_type(ctx, "Int", sf, true)),
            TypeIdentifier::BigInt => InputType::object(update_operations_object_type(ctx, "BigInt", sf, true)),
            TypeIdentifier::String => InputType::object(update_operations_object_type(ctx, "String", sf, false)),
            TypeIdentifier::Boolean => InputType::object(update_operations_object_type(ctx, "Bool", sf, false)),
            TypeIdentifier::Enum(enum_id) => {
                let enum_name = ctx.internal_data_model.walk(enum_id).name();
                InputType::object(update_operations_object_type(
                    ctx,
                    &format!("Enum{enum_name}"),
                    sf,
                    false,
                ))
            }
            TypeIdentifier::Json => map_scalar_input_type_for_field(ctx, sf),
            TypeIdentifier::DateTime => InputType::object(update_operations_object_type(ctx, "DateTime", sf, false)),
            TypeIdentifier::UUID => InputType::object(update_operations_object_type(ctx, "Uuid", sf, false)),
            TypeIdentifier::Xml => InputType::object(update_operations_object_type(ctx, "Xml", sf, false)),
            TypeIdentifier::Bytes => InputType::object(update_operations_object_type(ctx, "Bytes", sf, false)),

            TypeIdentifier::Unsupported => unreachable!("No unsupported field should reach that path"),
        };

        let has_adv_json = ctx.has_capability(ConnectorCapability::AdvancedJsonNullability);
        match sf.type_identifier() {
            TypeIdentifier::Json if has_adv_json => {
                let enum_type = InputType::enum_type(json_null_input_enum(ctx, !sf.is_required()));
                let input_field = input_field(sf.name(), vec![enum_type, base_update_type], None);

                input_field.optional()
            }

            _ => {
                let types = vec![map_scalar_input_type_for_field(ctx, sf), base_update_type];

                let input_field = input_field(sf.name(), types, None);
                input_field.optional().nullable_if(!sf.is_required())
            }
        }
    }

    fn map_scalar_list(&self, ctx: &mut BuilderContext, sf: &ScalarFieldRef) -> InputField {
        let list_input_type = map_scalar_input_type(ctx, &sf.type_identifier(), sf.is_list());
        let ident = Identifier::new(
            format!("{}Update{}Input", sf.container().name(), sf.name()),
            PRISMA_NAMESPACE,
        );

        let input_object = match ctx.get_input_type(&ident) {
            Some(t) => t,
            None => {
                let mut object_fields = vec![input_field(operations::SET, list_input_type.clone(), None).optional()];

                // Todo this capability looks wrong to me.
                if ctx.has_capability(ConnectorCapability::EnumArrayPush) {
                    object_fields.push(
                        input_field(
                            operations::PUSH,
                            vec![
                                list_input_type.clone(),
                                map_scalar_input_type(ctx, &sf.type_identifier(), false),
                            ],
                            None,
                        )
                        .optional(),
                    )
                }

                let mut input_object = input_object_type(ident.clone(), object_fields);
                input_object.require_exactly_one_field();

                let input_object = Arc::new(input_object);
                ctx.cache_input_type(ident, input_object.clone());

                Arc::downgrade(&input_object)
            }
        };

        let input_type = InputType::object(input_object);
        input_field(sf.name(), vec![input_type, list_input_type], None).optional()
    }

    fn map_relation(&self, ctx: &mut BuilderContext, rf: &RelationFieldRef) -> InputField {
        let related_model = rf.related_model();
        let related_field = rf.related_field();

        // Compute input object name
        let arity_part = match (rf.is_list(), rf.is_required()) {
            (true, _) => "Many",
            (false, true) => "OneRequired",
            (false, false) => "One",
        };

        let without_part = format!("Without{}", capitalize(related_field.name()));
        let unchecked_part = if self.unchecked { "Unchecked" } else { "" };
        let ident = Identifier::new(
            format!(
                "{}{}Update{}{}NestedInput",
                related_model.name(),
                unchecked_part,
                arity_part,
                without_part
            ),
            PRISMA_NAMESPACE,
        );

        let input_object = match ctx.get_input_type(&ident) {
            Some(t) => t,
            None => {
                let input_object = Arc::new(init_input_object_type(ident.clone()));
                ctx.cache_input_type(ident, input_object.clone());

                // Enqueue the nested update input for its fields to be
                // created at a later point, to avoid recursing too deep
                // (that has caused stack overflows on large schemas in
                // the past).
                ctx.nested_update_inputs_queue
                    .push((Arc::clone(&input_object), rf.clone()));

                Arc::downgrade(&input_object)
            }
        };

        input_field(rf.name(), InputType::object(input_object), None).optional()
    }

    fn map_composite(&self, ctx: &mut BuilderContext, cf: &CompositeFieldRef) -> InputField {
        // Shorthand object (equivalent to the "set" operation).
        let shorthand_type = InputType::Object(create::composite_create_object_type(ctx, cf));

        // Operation envelope object.
        let envelope_type = InputType::Object(composite_update_envelope_object_type(ctx, cf));

        let mut input_types = vec![envelope_type, shorthand_type.clone()];

        if cf.is_list() {
            input_types.push(InputType::list(shorthand_type));
        }

        input_field(cf.name(), input_types, None)
            .nullable_if(cf.is_optional() && !cf.is_list())
            .optional()
    }
}

fn update_operations_object_type(
    ctx: &mut BuilderContext,
    prefix: &str,
    sf: &ScalarFieldRef,
    with_number_operators: bool,
) -> InputObjectTypeWeakRef {
    // Different names are required to construct and cache different objects.
    // - "Nullable" affects the `set` operation (`set` is nullable)
    let nullable = if !sf.is_required() { "Nullable" } else { "" };
    let ident = Identifier::new(
        format!("{nullable}{prefix}FieldUpdateOperationsInput"),
        PRISMA_NAMESPACE,
    );
    return_cached_input!(ctx, &ident);

    let mut obj = init_input_object_type(ident.clone());
    obj.require_exactly_one_field();

    let obj = Arc::new(obj);
    ctx.cache_input_type(ident, obj.clone());

    let typ = map_scalar_input_type_for_field(ctx, sf);
    let mut fields = vec![input_field(operations::SET, typ.clone(), None)
        .optional()
        .nullable_if(!sf.is_required())];

    if with_number_operators {
        fields.push(input_field(operations::INCREMENT, typ.clone(), None).optional());
        fields.push(input_field(operations::DECREMENT, typ.clone(), None).optional());
        fields.push(input_field(operations::MULTIPLY, typ.clone(), None).optional());
        fields.push(input_field(operations::DIVIDE, typ, None).optional());
    }

    if ctx.has_capability(ConnectorCapability::UndefinedType) && !sf.is_required() {
        fields.push(input_field(operations::UNSET, InputType::boolean(), None).optional());
    }

    obj.set_fields(fields);

    Arc::downgrade(&obj)
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
fn composite_update_envelope_object_type(ctx: &mut BuilderContext, cf: &CompositeFieldRef) -> InputObjectTypeWeakRef {
    let arity = if cf.is_optional() {
        "Nullable"
    } else if cf.is_list() {
        "List"
    } else {
        ""
    };

    let name = format!("{}{}UpdateEnvelopeInput", cf.typ().name, arity);
    let ident = Identifier::new(name, PRISMA_NAMESPACE);

    return_cached_input!(ctx, &ident);

    let mut input_object = init_input_object_type(ident.clone());
    input_object.require_exactly_one_field();
    input_object.set_tag(ObjectTag::CompositeEnvelope);

    let input_object = Arc::new(input_object);
    ctx.cache_input_type(ident, input_object.clone());

    let mut fields = vec![composite_set_update_input_field(ctx, cf)];

    append_opt(&mut fields, composite_update_input_field(ctx, cf));
    append_opt(&mut fields, composite_push_update_input_field(ctx, cf));
    append_opt(&mut fields, composite_upsert_update_input_field(ctx, cf));
    append_opt(&mut fields, composite_update_many_update_input_field(ctx, cf));
    append_opt(&mut fields, composite_delete_many_update_input_field(ctx, cf));
    append_opt(&mut fields, composite_unset_update_input_field(cf));

    input_object.set_fields(fields);

    Arc::downgrade(&input_object)
}

/// Builds the `update` input object type. Should be used in the envelope type.
fn composite_update_object_type(ctx: &mut BuilderContext, cf: &CompositeFieldRef) -> InputObjectTypeWeakRef {
    let name = format!("{}UpdateInput", cf.typ().name);

    let ident = Identifier::new(name, PRISMA_NAMESPACE);
    return_cached_input!(ctx, &ident);

    let mut input_object = init_input_object_type(ident.clone());
    input_object.set_min_fields(1);

    let input_object = Arc::new(input_object);
    ctx.cache_input_type(ident, input_object.clone());

    let mapper = UpdateDataInputFieldMapper::new_checked();
    let fields = cf.typ().fields().collect::<Vec<_>>();
    let fields = mapper.map_all(ctx, &fields);

    input_object.set_fields(fields);

    Arc::downgrade(&input_object)
}

// Builds an `update` input field. Should only be used in the envelope type.
fn composite_update_input_field(ctx: &mut BuilderContext, cf: &CompositeFieldRef) -> Option<InputField> {
    if cf.is_required() {
        let update_object_type = composite_update_object_type(ctx, cf);

        Some(input_field(operations::UPDATE, InputType::Object(update_object_type), None).optional())
    } else {
        None
    }
}

// Builds an `unset` input field. Should only be used in the envelope type.
fn composite_unset_update_input_field(cf: &CompositeFieldRef) -> Option<InputField> {
    if cf.is_optional() {
        Some(input_field(operations::UNSET, InputType::boolean(), None).optional())
    } else {
        None
    }
}

// Builds an `set` input field. Should only be used in the envelope type.
fn composite_set_update_input_field(ctx: &mut BuilderContext, cf: &CompositeFieldRef) -> InputField {
    let set_object_type = InputType::Object(create::composite_create_object_type(ctx, cf));

    let mut input_types = vec![set_object_type.clone()];

    if cf.is_list() {
        input_types.push(InputType::list(set_object_type));
    }

    input_field(operations::SET, input_types, None)
        .nullable_if(!cf.is_required() && !cf.is_list())
        .optional()
}

// Builds an `push` input field. Should only be used in the envelope type.
fn composite_push_update_input_field(ctx: &mut BuilderContext, cf: &CompositeFieldRef) -> Option<InputField> {
    if cf.is_list() {
        let set_object_type = InputType::Object(create::composite_create_object_type(ctx, cf));
        let input_types = vec![set_object_type.clone(), InputType::list(set_object_type)];

        Some(input_field(operations::PUSH, input_types, None).optional())
    } else {
        None
    }
}

/// Builds the `upsert` input object type. Should only be used in the envelope type.
fn composite_upsert_object_type(ctx: &mut BuilderContext, cf: &CompositeFieldRef) -> InputObjectTypeWeakRef {
    let name = format!("{}UpsertInput", cf.typ().name);

    let ident = Identifier::new(name, PRISMA_NAMESPACE);
    return_cached_input!(ctx, &ident);

    let mut input_object = init_input_object_type(ident.clone());
    input_object.set_tag(ObjectTag::CompositeEnvelope);

    let input_object = Arc::new(input_object);

    ctx.cache_input_type(ident, input_object.clone());

    let update_object_type = composite_update_object_type(ctx, cf);
    let update_field = input_field(operations::UPDATE, InputType::Object(update_object_type), None);
    let set_field = composite_set_update_input_field(ctx, cf).required();

    let fields = vec![set_field, update_field];

    input_object.set_fields(fields);

    Arc::downgrade(&input_object)
}

// Builds an `upsert` input field. Should only be used in the envelope type.
fn composite_upsert_update_input_field(ctx: &mut BuilderContext, cf: &CompositeFieldRef) -> Option<InputField> {
    if cf.is_optional() {
        let upsert_object_type = InputType::Object(composite_upsert_object_type(ctx, cf));

        Some(input_field(operations::UPSERT, upsert_object_type, None).optional())
    } else {
        None
    }
}

fn composite_update_many_object_type(ctx: &mut BuilderContext, cf: &CompositeFieldRef) -> InputObjectTypeWeakRef {
    let name = format!("{}UpdateManyInput", cf.typ().name);

    let ident = Identifier::new(name, PRISMA_NAMESPACE);
    return_cached_input!(ctx, &ident);

    let mut input_object = init_input_object_type(ident.clone());
    input_object.set_tag(ObjectTag::CompositeEnvelope);

    let input_object = Arc::new(input_object);

    ctx.cache_input_type(ident, input_object.clone());

    let where_object_type = objects::filter_objects::where_object_type(ctx, &cf.typ());
    let where_field = input_field(args::WHERE, InputType::object(where_object_type), None);

    let update_object_type = composite_update_object_type(ctx, cf);
    let data_field = input_field(args::DATA, InputType::Object(update_object_type), None);

    let fields = vec![where_field, data_field];

    input_object.set_fields(fields);

    Arc::downgrade(&input_object)
}

fn composite_delete_many_object_type(ctx: &mut BuilderContext, cf: &CompositeFieldRef) -> InputObjectTypeWeakRef {
    let name = format!("{}DeleteManyInput", cf.typ().name);

    let ident = Identifier::new(name, PRISMA_NAMESPACE);
    return_cached_input!(ctx, &ident);

    let mut input_object = init_input_object_type(ident.clone());
    input_object.set_tag(ObjectTag::CompositeEnvelope);

    let input_object = Arc::new(input_object);

    ctx.cache_input_type(ident, input_object.clone());

    let where_object_type = objects::filter_objects::where_object_type(ctx, &cf.typ());
    let where_field = input_field(args::WHERE, InputType::object(where_object_type), None);

    let fields = vec![where_field];

    input_object.set_fields(fields);

    Arc::downgrade(&input_object)
}

// Builds an `updateMany` input field. Should only be used in the envelope type.
fn composite_update_many_update_input_field(ctx: &mut BuilderContext, cf: &CompositeFieldRef) -> Option<InputField> {
    if cf.is_list() {
        let update_many = InputType::Object(composite_update_many_object_type(ctx, cf));

        Some(input_field(operations::UPDATE_MANY, update_many, None).optional())
    } else {
        None
    }
}

// Builds a `deleteMany` input field. Should only be used in the envelope type.
fn composite_delete_many_update_input_field(ctx: &mut BuilderContext, cf: &CompositeFieldRef) -> Option<InputField> {
    if cf.is_list() {
        let delete_many = InputType::Object(composite_delete_many_object_type(ctx, cf));

        Some(input_field(operations::DELETE_MANY, delete_many, None).optional())
    } else {
        None
    }
}

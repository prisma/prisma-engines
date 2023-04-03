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
    fn map_scalar(&self, ctx: &mut BuilderContext<'_>, sf: &ScalarFieldRef) -> InputField {
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
                let input_field = input_field(ctx, sf.name(), vec![enum_type, base_update_type], None);

                input_field.optional()
            }

            _ => {
                let types = vec![map_scalar_input_type_for_field(ctx, sf), base_update_type];

                let input_field = input_field(ctx, sf.name(), types, None);
                input_field.optional().nullable_if(!sf.is_required(), &mut ctx.db)
            }
        }
    }

    fn map_scalar_list(&self, ctx: &mut BuilderContext<'_>, sf: &ScalarFieldRef) -> InputField {
        let list_input_type = map_scalar_input_type(ctx, &sf.type_identifier(), sf.is_list());
        let ident = Identifier::new_prisma(IdentifierType::ScalarListUpdateInput(sf.clone()));

        let input_object = match ctx.get_input_type(&ident) {
            Some(t) => t,
            None => {
                let mut object_fields =
                    vec![input_field(ctx, operations::SET, list_input_type.clone(), None).optional()];

                // Todo this capability looks wrong to me.
                if ctx.has_capability(ConnectorCapability::EnumArrayPush) {
                    let map_scalar_type = map_scalar_input_type(ctx, &sf.type_identifier(), false);
                    object_fields.push(
                        input_field(ctx, operations::PUSH, [map_scalar_type, list_input_type.clone()], None).optional(),
                    )
                }

                let mut input_object = init_input_object_type(ident.clone());
                input_object.require_exactly_one_field();
                let id = ctx.cache_input_type(ident, input_object);
                for field in object_fields {
                    ctx.db.push_input_field(id, field);
                }
                id
            }
        };

        let input_type = InputType::object(input_object);
        input_field(ctx, sf.name(), vec![input_type, list_input_type], None).optional()
    }

    fn map_relation(&self, ctx: &mut BuilderContext<'_>, rf: &RelationFieldRef) -> InputField {
        let ident = Identifier::new_prisma(IdentifierType::RelationUpdateInput(
            rf.clone(),
            rf.related_field(),
            self.unchecked,
        ));

        let input_object = match ctx.get_input_type(&ident) {
            Some(t) => t,
            None => {
                let input_object = init_input_object_type(ident.clone());
                let id = ctx.cache_input_type(ident, input_object);

                // Enqueue the nested update input for its fields to be
                // created at a later point, to avoid recursing too deep
                // (that has caused stack overflows on large schemas in
                // the past).
                ctx.nested_update_inputs_queue.push((id, rf.clone()));
                id
            }
        };

        input_field(ctx, rf.name(), InputType::object(input_object), None).optional()
    }

    fn map_composite(&self, ctx: &mut BuilderContext<'_>, cf: &CompositeFieldRef) -> InputField {
        // Shorthand object (equivalent to the "set" operation).
        let shorthand_type = InputType::Object(create::composite_create_object_type(ctx, cf));

        // Operation envelope object.
        let envelope_type = InputType::Object(composite_update_envelope_object_type(ctx, cf));

        let mut input_types = vec![envelope_type, shorthand_type.clone()];

        if cf.is_list() {
            input_types.push(InputType::list(shorthand_type));
        }

        input_field(ctx, cf.name(), input_types, None)
            .nullable_if(cf.is_optional() && !cf.is_list(), &mut ctx.db)
            .optional()
    }
}

fn update_operations_object_type(
    ctx: &mut BuilderContext<'_>,
    prefix: &str,
    sf: &ScalarFieldRef,
    with_number_operators: bool,
) -> InputObjectTypeId {
    let ident = Identifier::new_prisma(IdentifierType::FieldUpdateOperationsInput(
        !sf.is_required(),
        prefix.to_owned(),
    ));
    return_cached_input!(ctx, &ident);

    let mut obj = init_input_object_type(ident.clone());
    obj.require_exactly_one_field();
    let id = ctx.cache_input_type(ident, obj);

    let typ = map_scalar_input_type_for_field(ctx, sf);

    let set_field = input_field(ctx, operations::SET, typ.clone(), None)
        .optional()
        .nullable_if(!sf.is_required(), &mut ctx.db);
    ctx.db.push_input_field(id, set_field);

    if with_number_operators {
        let increment_field = input_field(ctx, operations::INCREMENT, typ.clone(), None).optional();
        let decrement_field = input_field(ctx, operations::DECREMENT, typ.clone(), None).optional();
        let multiply_field = input_field(ctx, operations::MULTIPLY, typ.clone(), None).optional();
        let divide_field = input_field(ctx, operations::DIVIDE, typ, None).optional();
        ctx.db.extend_input_fields(
            id,
            &mut [increment_field, decrement_field, multiply_field, divide_field].into_iter(),
        );
    }

    if ctx.has_capability(ConnectorCapability::UndefinedType) && !sf.is_required() {
        let unset_field = input_field(ctx, operations::UNSET, InputType::boolean(), None).optional();
        ctx.db.push_input_field(id, unset_field);
    }

    id
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
fn composite_update_envelope_object_type(ctx: &mut BuilderContext<'_>, cf: &CompositeFieldRef) -> InputObjectTypeId {
    let ident = Identifier::new_prisma(IdentifierType::CompositeUpdateEnvelopeInput(cf.typ(), cf.arity()));

    return_cached_input!(ctx, &ident);

    let mut input_object = init_input_object_type(ident.clone());
    input_object.require_exactly_one_field();
    input_object.set_tag(ObjectTag::CompositeEnvelope);
    let id = ctx.cache_input_type(ident, input_object);

    let mut fields = vec![composite_set_update_input_field(ctx, cf)];

    append_opt(&mut fields, composite_update_input_field(ctx, cf));
    append_opt(&mut fields, composite_push_update_input_field(ctx, cf));
    append_opt(&mut fields, composite_upsert_update_input_field(ctx, cf));
    append_opt(&mut fields, composite_update_many_update_input_field(ctx, cf));
    append_opt(&mut fields, composite_delete_many_update_input_field(ctx, cf));
    append_opt(&mut fields, composite_unset_update_input_field(ctx, cf));
    for field in fields {
        ctx.db.push_input_field(id, field);
    }

    id
}

/// Builds the `update` input object type. Should be used in the envelope type.
fn composite_update_object_type(ctx: &mut BuilderContext<'_>, cf: &CompositeFieldRef) -> InputObjectTypeId {
    let ident = Identifier::new_prisma(IdentifierType::CompositeUpdateInput(cf.typ()));

    return_cached_input!(ctx, &ident);

    let mut input_object = init_input_object_type(ident.clone());
    input_object.set_min_fields(1);
    let id = ctx.cache_input_type(ident, input_object);

    let mapper = UpdateDataInputFieldMapper::new_checked();
    let typ = cf.typ();
    for field in typ.fields() {
        mapper.map_field(ctx, id, &field)
    }

    id
}

// Builds an `update` input field. Should only be used in the envelope type.
fn composite_update_input_field(ctx: &mut BuilderContext<'_>, cf: &CompositeFieldRef) -> Option<InputField> {
    if cf.is_required() {
        let update_object_type = composite_update_object_type(ctx, cf);

        Some(input_field(ctx, operations::UPDATE, InputType::Object(update_object_type), None).optional())
    } else {
        None
    }
}

// Builds an `unset` input field. Should only be used in the envelope type.
fn composite_unset_update_input_field(ctx: &mut BuilderContext<'_>, cf: &CompositeFieldRef) -> Option<InputField> {
    if cf.is_optional() {
        Some(input_field(ctx, operations::UNSET, InputType::boolean(), None).optional())
    } else {
        None
    }
}

// Builds an `set` input field. Should only be used in the envelope type.
fn composite_set_update_input_field(ctx: &mut BuilderContext<'_>, cf: &CompositeFieldRef) -> InputField {
    let set_object_type = InputType::Object(create::composite_create_object_type(ctx, cf));

    let mut input_types = vec![set_object_type.clone()];

    if cf.is_list() {
        input_types.push(InputType::list(set_object_type));
    }

    input_field(ctx, operations::SET, input_types, None)
        .nullable_if(!cf.is_required() && !cf.is_list(), &mut ctx.db)
        .optional()
}

// Builds an `push` input field. Should only be used in the envelope type.
fn composite_push_update_input_field(ctx: &mut BuilderContext<'_>, cf: &CompositeFieldRef) -> Option<InputField> {
    if cf.is_list() {
        let set_object_type = InputType::Object(create::composite_create_object_type(ctx, cf));
        let input_types = vec![set_object_type.clone(), InputType::list(set_object_type)];

        Some(input_field(ctx, operations::PUSH, input_types, None).optional())
    } else {
        None
    }
}

/// Builds the `upsert` input object type. Should only be used in the envelope type.
fn composite_upsert_object_type(ctx: &mut BuilderContext<'_>, cf: &CompositeFieldRef) -> InputObjectTypeId {
    let ident = Identifier::new_prisma(IdentifierType::CompositeUpsertObjectInput(cf.typ()));

    return_cached_input!(ctx, &ident);

    let mut input_object = init_input_object_type(ident.clone());
    input_object.set_tag(ObjectTag::CompositeEnvelope);
    let id = ctx.cache_input_type(ident, input_object);

    let update_object_type = composite_update_object_type(ctx, cf);
    let update_field = input_field(ctx, operations::UPDATE, InputType::Object(update_object_type), None);
    let set_field = composite_set_update_input_field(ctx, cf).required();

    ctx.db
        .extend_input_fields(id, &mut [set_field, update_field].into_iter());
    id
}

// Builds an `upsert` input field. Should only be used in the envelope type.
fn composite_upsert_update_input_field(ctx: &mut BuilderContext<'_>, cf: &CompositeFieldRef) -> Option<InputField> {
    if cf.is_optional() {
        let upsert_object_type = InputType::Object(composite_upsert_object_type(ctx, cf));

        Some(input_field(ctx, operations::UPSERT, upsert_object_type, None).optional())
    } else {
        None
    }
}

fn composite_update_many_object_type(ctx: &mut BuilderContext<'_>, cf: &CompositeFieldRef) -> InputObjectTypeId {
    let ident = Identifier::new_prisma(IdentifierType::CompositeUpdateManyInput(cf.typ()));

    return_cached_input!(ctx, &ident);

    let mut input_object = init_input_object_type(ident.clone());
    input_object.set_tag(ObjectTag::CompositeEnvelope);
    let id = ctx.cache_input_type(ident, input_object);

    let where_object_type = objects::filter_objects::where_object_type(ctx, cf.typ());
    let where_field = input_field(ctx, args::WHERE, InputType::object(where_object_type), None);

    let update_object_type = composite_update_object_type(ctx, cf);
    let data_field = input_field(ctx, args::DATA, InputType::Object(update_object_type), None);

    ctx.db
        .extend_input_fields(id, &mut [where_field, data_field].into_iter());
    id
}

fn composite_delete_many_object_type(ctx: &mut BuilderContext<'_>, cf: &CompositeFieldRef) -> InputObjectTypeId {
    let ident = Identifier::new_prisma(IdentifierType::CompositeDeleteManyInput(cf.typ()));

    return_cached_input!(ctx, &ident);

    let mut input_object = init_input_object_type(ident.clone());
    input_object.set_tag(ObjectTag::CompositeEnvelope);
    let id = ctx.cache_input_type(ident, input_object);

    let where_object_type = objects::filter_objects::where_object_type(ctx, cf.typ());
    let where_field = input_field(ctx, args::WHERE, InputType::object(where_object_type), None);
    ctx.db.push_input_field(id, where_field);
    id
}

// Builds an `updateMany` input field. Should only be used in the envelope type.
fn composite_update_many_update_input_field(
    ctx: &mut BuilderContext<'_>,
    cf: &CompositeFieldRef,
) -> Option<InputField> {
    if cf.is_list() {
        let update_many = InputType::Object(composite_update_many_object_type(ctx, cf));

        Some(input_field(ctx, operations::UPDATE_MANY, update_many, None).optional())
    } else {
        None
    }
}

// Builds a `deleteMany` input field. Should only be used in the envelope type.
fn composite_delete_many_update_input_field(
    ctx: &mut BuilderContext<'_>,
    cf: &CompositeFieldRef,
) -> Option<InputField> {
    if cf.is_list() {
        let delete_many = InputType::Object(composite_delete_many_object_type(ctx, cf));

        Some(input_field(ctx, operations::DELETE_MANY, delete_many, None).optional())
    } else {
        None
    }
}

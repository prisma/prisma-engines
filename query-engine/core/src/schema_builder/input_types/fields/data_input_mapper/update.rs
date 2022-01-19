use super::*;
use crate::constants::*;
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
        let base_update_type = match &sf.type_identifier {
            TypeIdentifier::Float => InputType::object(update_operations_object_type(ctx, "Float", sf, true)),
            TypeIdentifier::Decimal => InputType::object(update_operations_object_type(ctx, "Decimal", sf, true)),
            TypeIdentifier::Int => InputType::object(update_operations_object_type(ctx, "Int", sf, true)),
            TypeIdentifier::BigInt => InputType::object(update_operations_object_type(ctx, "BigInt", sf, true)),
            TypeIdentifier::String => InputType::object(update_operations_object_type(ctx, "String", sf, false)),
            TypeIdentifier::Boolean => InputType::object(update_operations_object_type(ctx, "Bool", sf, false)),
            TypeIdentifier::Enum(e) => {
                InputType::object(update_operations_object_type(ctx, &format!("Enum{}", e), sf, false))
            }
            TypeIdentifier::Json => map_scalar_input_type_for_field(ctx, sf),
            TypeIdentifier::DateTime => InputType::object(update_operations_object_type(ctx, "DateTime", sf, false)),
            TypeIdentifier::UUID => InputType::object(update_operations_object_type(ctx, "Uuid", sf, false)),
            TypeIdentifier::Xml => InputType::object(update_operations_object_type(ctx, "Xml", sf, false)),
            TypeIdentifier::Bytes => InputType::object(update_operations_object_type(ctx, "Bytes", sf, false)),

            TypeIdentifier::Unsupported => unreachable!("No unsupported field should reach that path"),
        };

        let has_adv_json = ctx.has_capability(ConnectorCapability::AdvancedJsonNullability);
        match &sf.type_identifier {
            TypeIdentifier::Json if has_adv_json => {
                let enum_type = json_null_input_enum(!sf.is_required());
                let input_field = input_field(
                    sf.name.clone(),
                    vec![InputType::Enum(enum_type), base_update_type],
                    None,
                );

                input_field.optional()
            }

            _ => {
                let types = vec![map_scalar_input_type_for_field(ctx, sf), base_update_type];

                let input_field = input_field(sf.name.clone(), types, None);
                input_field.optional().nullable_if(!sf.is_required())
            }
        }
    }

    fn map_scalar_list(&self, ctx: &mut BuilderContext, sf: &ScalarFieldRef) -> InputField {
        let list_input_type = map_scalar_input_type(ctx, &sf.type_identifier, sf.is_list());
        let ident = Identifier::new(
            format!("{}Update{}Input", sf.container.name(), sf.name),
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
                                map_scalar_input_type(ctx, &sf.type_identifier, false),
                                list_input_type.clone(),
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
        input_field(sf.name.clone(), vec![input_type, list_input_type], None).optional()
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

        let without_part = format!("Without{}", capitalize(&related_field.name));
        let unchecked_part = if self.unchecked { "Unchecked" } else { "" };
        let ident = Identifier::new(
            format!(
                "{}{}Update{}{}Input",
                related_model.name, unchecked_part, arity_part, without_part
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
                    .push((Arc::clone(&input_object), Arc::clone(&rf)));

                Arc::downgrade(&input_object)
            }
        };

        input_field(rf.name.clone(), InputType::object(input_object), None).optional()
    }

    fn map_composite(&self, ctx: &mut BuilderContext, cf: &CompositeFieldRef) -> InputField {
        todo!()
    }
}

fn update_operations_object_type(
    ctx: &mut BuilderContext,
    prefix: &str,
    field: &ScalarFieldRef,
    with_number_operators: bool,
) -> InputObjectTypeWeakRef {
    // Nullability is important for the `set` operation, so we need to
    // construct and cache different objects to reflect that.
    let nullable = if field.is_required() { "" } else { "Nullable" };
    let ident = Identifier::new(
        format!("{}{}FieldUpdateOperationsInput", nullable, prefix),
        PRISMA_NAMESPACE,
    );
    return_cached_input!(ctx, &ident);

    let mut obj = init_input_object_type(ident.clone());
    obj.require_exactly_one_field();

    let obj = Arc::new(obj);
    ctx.cache_input_type(ident, obj.clone());

    let typ = map_scalar_input_type_for_field(ctx, field);
    let mut fields = vec![input_field(operations::SET, typ.clone(), None)
        .optional()
        .nullable_if(!field.is_required())];

    if with_number_operators {
        fields.push(input_field(operations::INCREMENT, typ.clone(), None).optional());
        fields.push(input_field(operations::DECREMENT, typ.clone(), None).optional());
        fields.push(input_field(operations::MULTIPLY, typ.clone(), None).optional());
        fields.push(input_field(operations::DIVIDE, typ, None).optional());
    }

    obj.set_fields(fields);

    Arc::downgrade(&obj)
}

use super::*;
use crate::constants::*;
use prisma_models::CompositeFieldRef;

pub(crate) struct CreateDataInputFieldMapper {
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

impl DataInputFieldMapper for CreateDataInputFieldMapper {
    fn map_scalar(&self, ctx: &mut BuilderContext, sf: &ScalarFieldRef) -> InputField {
        let typ = map_scalar_input_type_for_field(ctx, &sf);
        let supports_advanced_json = ctx.has_capability(ConnectorCapability::AdvancedJsonNullability);

        match &sf.type_identifier {
            TypeIdentifier::Json if supports_advanced_json => {
                let enum_type = json_null_input_enum(!sf.is_required());

                input_field(
                    sf.name.clone(),
                    vec![InputType::Enum(enum_type), typ],
                    sf.default_value.clone(),
                )
                .optional_if(!sf.is_required() || sf.default_value.is_some() || sf.is_updated_at)
            }

            _ => input_field(sf.name.clone(), typ, sf.default_value.clone())
                .optional_if(!sf.is_required() || sf.default_value.is_some() || sf.is_updated_at)
                .nullable_if(!sf.is_required()),
        }
    }

    fn map_scalar_list(&self, ctx: &mut BuilderContext, sf: &ScalarFieldRef) -> InputField {
        let typ = map_scalar_input_type_for_field(ctx, &sf);
        let ident = Identifier::new(
            format!("{}Create{}Input", sf.container.name(), sf.name),
            PRISMA_NAMESPACE,
        );

        let input_object = match ctx.get_input_type(&ident) {
            Some(cached) => cached,
            None => {
                let object_fields = vec![input_field(operations::SET, typ.clone(), None)];
                let mut input_object = input_object_type(ident.clone(), object_fields);
                input_object.require_exactly_one_field();

                let input_object = Arc::new(input_object);
                ctx.cache_input_type(ident, input_object.clone());

                Arc::downgrade(&input_object)
            }
        };

        let input_type = InputType::object(input_object);

        // Shorthand type (`list_field: <typ>`) + full object (`list_field: { set: { <typ> }}`)
        input_field(sf.name.clone(), vec![input_type, typ], None).optional()
    }

    fn map_relation(&self, ctx: &mut BuilderContext, rf: &RelationFieldRef) -> InputField {
        let related_model = rf.related_model();
        let related_field = rf.related_field();

        // Compute input object name
        let arity_part = if rf.is_list() { "NestedMany" } else { "NestedOne" };
        let without_part = format!("Without{}", capitalize(&related_field.name));
        let unchecked_part = if self.unchecked { "Unchecked" } else { "" };
        let ident = Identifier::new(
            format!(
                "{}{}Create{}{}Input",
                related_model.name, unchecked_part, arity_part, without_part
            ),
            PRISMA_NAMESPACE,
        );

        let input_object = match ctx.get_input_type(&ident) {
            Some(t) => t,
            None => {
                let input_object = Arc::new(init_input_object_type(ident.clone()));
                ctx.cache_input_type(ident, input_object.clone());

                // Enqueue the nested create input for its fields to be
                // created at a later point, to avoid recursing too deep
                // (that has caused stack overflows on large schemas in
                // the past).
                ctx.nested_create_inputs_queue
                    .push((Arc::clone(&input_object), Arc::clone(&rf)));

                Arc::downgrade(&input_object)
            }
        };

        // If all backing scalars of a relation have a default, the entire relation is optional on create, even if the relation field itself is optional.
        let all_required_scalar_fields_have_defaults = rf
            .linking_fields()
            .as_scalar_fields()
            .expect("Expected linking fields to be scalar.")
            .into_iter()
            .all(|scalar_field| scalar_field.default_value.is_some());

        let input_field = input_field(rf.name.clone(), InputType::object(input_object), None);

        if rf.is_required() && !all_required_scalar_fields_have_defaults {
            input_field
        } else {
            input_field.optional()
        }
    }

    fn map_composite(&self, ctx: &mut BuilderContext, cf: &CompositeFieldRef) -> InputField {
        todo!()
    }
}

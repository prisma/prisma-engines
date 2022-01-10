use super::*;
use crate::constants::*;
use prisma_models::CompositeFieldRef;

#[derive(Default)]
pub(crate) struct CompositeDataInputFieldMapper {
    is_create: bool,
}

impl CompositeDataInputFieldMapper {
    fn for_create_operations() -> Self {
        Self { is_create: true }
    }
}

impl DataInputFieldMapper for CompositeDataInputFieldMapper {
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
        unreachable!("Composites do not support relation fields.");
    }

    fn map_composite(&self, ctx: &mut BuilderContext, cf: &CompositeFieldRef) -> InputField {
        todo!()
    }
}

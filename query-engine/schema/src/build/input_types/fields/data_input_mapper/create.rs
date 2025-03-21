use super::*;
use constants::*;
use query_structure::CompositeFieldRef;

pub(crate) struct CreateDataInputFieldMapper {
    unchecked: bool,
}

impl CreateDataInputFieldMapper {
    pub fn new_checked() -> Self {
        Self { unchecked: false }
    }

    pub fn new_unchecked() -> Self {
        Self { unchecked: true }
    }
}

impl DataInputFieldMapper for CreateDataInputFieldMapper {
    fn map_scalar<'a>(&self, ctx: &'a QuerySchema, sf: ScalarFieldRef) -> InputField<'a> {
        let typ = map_scalar_input_type_for_field(ctx, &sf);
        let supports_advanced_json = ctx.has_capability(ConnectorCapability::AdvancedJsonNullability);

        match &sf.type_identifier() {
            TypeIdentifier::Json if supports_advanced_json => {
                let enum_type = InputType::enum_type(json_null_input_enum(!sf.is_required()));

                input_field(sf.name().to_owned(), vec![enum_type, typ], sf.default_value())
                    .optional_if(!sf.is_required() || sf.default_value().is_some() || sf.is_updated_at())
            }

            _ => input_field(sf.name().to_owned(), vec![typ], sf.default_value())
                .optional_if(!sf.is_required() || sf.default_value().is_some() || sf.is_updated_at())
                .nullable_if(!sf.is_required()),
        }
    }

    fn map_scalar_list<'a>(&self, ctx: &'a QuerySchema, sf: ScalarFieldRef) -> InputField<'a> {
        let typ = map_scalar_input_type_for_field(ctx, &sf);
        let cloned_typ = typ.clone();
        let ident = Identifier::new_prisma(IdentifierType::CreateOneScalarList(sf.clone()));

        let mut input_object = init_input_object_type(ident);
        input_object.set_container(sf.container());
        input_object.require_exactly_one_field();
        input_object.set_fields(move || vec![simple_input_field(operations::SET, cloned_typ.clone(), None)]);

        let input_type = InputType::object(input_object);

        // Shorthand type (`list_field: <typ>`) + full object (`list_field: { set: { <typ> }}`)
        input_field(sf.name().to_owned(), vec![input_type, typ], sf.default_value()).optional()
    }

    fn map_relation<'a>(&self, ctx: &'a QuerySchema, rf: RelationFieldRef) -> InputField<'a> {
        let ident = Identifier::new_prisma(IdentifierType::RelationCreateInput(
            rf.clone(),
            rf.related_field(),
            self.unchecked,
        ));

        let cloned_rf = rf.clone();
        let mut input_object = init_input_object_type(ident);
        input_object.set_container(rf.related_model());
        input_object.set_fields(move || {
            let rf = &cloned_rf;
            let mut fields = vec![];

            if rf.related_model().supports_create_operation() {
                fields.push(input_fields::nested_create_one_input_field(ctx, rf.clone()));

                append_opt(
                    &mut fields,
                    input_fields::nested_connect_or_create_field(ctx, rf.clone()),
                );
                append_opt(
                    &mut fields,
                    input_fields::nested_create_many_input_field(ctx, rf.clone()),
                );
            }

            fields.push(input_fields::nested_connect_input_field(ctx, rf));
            fields
        });

        // If all backing scalars of a relation have a default, the entire relation is optional on create, even if the relation field itself is optional.
        let all_required_scalar_fields_have_defaults = rf
            .linking_fields()
            .as_scalar_fields()
            .expect("Expected linking fields to be scalar.")
            .into_iter()
            .all(|scalar_field| scalar_field.default_value().is_some());

        let input_field = simple_input_field(rf.name().to_owned(), InputType::object(input_object), None);

        if rf.is_required() && !all_required_scalar_fields_have_defaults {
            input_field
        } else {
            input_field.optional()
        }
    }

    fn map_composite<'a>(&self, ctx: &'a QuerySchema, cf: CompositeFieldRef) -> InputField<'a> {
        // Shorthand object (just the plain create object for the composite).
        let shorthand_type = InputType::Object(composite_create_object_type(ctx, cf.clone()));

        // Operation envelope object.
        let envelope_type = InputType::Object(composite_create_envelope_object_type(ctx, cf.clone()));

        // If the composite field in _not_ on a model, then it's nested and we're skipping the create envelope for now.
        // (This allows us to simplify the parsing code for now.)
        let mut input_types = if cf.container().as_model().is_some() {
            vec![envelope_type, shorthand_type.clone()]
        } else {
            vec![shorthand_type.clone()]
        };

        if cf.is_list() {
            input_types.push(InputType::list(shorthand_type));
        }

        input_field(cf.name().to_owned(), input_types, None)
            .nullable_if(!cf.is_required() && !cf.is_list())
            .optional_if(!cf.is_required())
    }
}

/// Build an operation envelope object type for composite creates.
/// An operation envelope is an object that encapsulates the possible operations, like:
/// ```text
/// cf_field: { // this is the envelope object
///   set: { ... create type ... }
///   ... more ops ...
/// }
/// ```
fn composite_create_envelope_object_type(ctx: &'_ QuerySchema, cf: CompositeFieldRef) -> InputObjectType<'_> {
    let ident = Identifier::new_prisma(IdentifierType::CompositeCreateEnvelopeInput(cf.typ(), cf.arity()));
    let cf_is_list = cf.is_list();
    let cf_is_required = cf.is_required();

    let mut input_object = init_input_object_type(ident);
    input_object.set_container(cf.typ());
    input_object.require_exactly_one_field();
    input_object.set_tag(ObjectTag::CompositeEnvelope);
    input_object.set_fields(move || {
        let create_input = InputType::Object(composite_create_object_type(ctx, cf.clone()));
        let mut input_types = vec![create_input.clone()];

        if cf_is_list {
            input_types.push(InputType::list(create_input));
        }

        let set_field = input_field("set", input_types, None)
            .nullable_if(!cf_is_required && !cf_is_list)
            .optional();

        vec![set_field]
    });
    input_object
}

pub(crate) fn composite_create_object_type(ctx: &'_ QuerySchema, cf: CompositeFieldRef) -> InputObjectType<'_> {
    // It's called "Create" input because it's used across multiple create-type operations, not only "set".
    let ident = Identifier::new_prisma(IdentifierType::CompositeCreateInput(cf.typ()));

    let mut input_object = init_input_object_type(ident);
    input_object.set_container(cf.container());
    input_object.set_fields(move || {
        let mapper = CreateDataInputFieldMapper::new_checked();
        let typ = cf.typ();
        let mut fields = typ.fields();
        mapper.map_all(ctx, &mut fields)
    });
    input_object
}

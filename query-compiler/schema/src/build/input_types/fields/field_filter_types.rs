use super::{field_ref_type::WithFieldRefInputExt, objects::*, *};
use constants::{aggregations, filters};
use psl::datamodel_connector::ConnectorCapability;
use query_structure::{CompositeFieldRef, DefaultKind, NativeTypeInstance, PrismaValue};

/// Builds filter types for the given model field.
pub(crate) fn get_field_filter_types(
    ctx: &'_ QuerySchema,
    field: ModelField,
    include_aggregates: bool,
) -> Vec<InputType<'_>> {
    match field {
        ModelField::Relation(rf) if rf.is_list() => {
            vec![InputType::object(to_many_relation_filter_object(ctx, rf))]
        }

        ModelField::Relation(rf) => {
            vec![
                InputType::object(to_one_relation_filter_object(ctx, rf.clone())),
                to_one_relation_filter_shorthand_types(ctx, &rf),
            ]
        }

        ModelField::Composite(cf) if cf.is_list() => vec![
            InputType::object(to_many_composite_filter_object(ctx, cf.clone())),
            InputType::list(to_one_composite_filter_shorthand_types(ctx, cf)),
        ],

        ModelField::Composite(cf) => vec![
            InputType::object(to_one_composite_filter_object(ctx, cf.clone())),
            to_one_composite_filter_shorthand_types(ctx, cf),
        ],

        ModelField::Scalar(sf) if field.is_list() => vec![InputType::object(scalar_list_filter_type(ctx, sf))],

        ModelField::Scalar(sf) => {
            let mut types = vec![InputType::object(full_scalar_filter_type(
                ctx,
                sf.type_identifier(),
                sf.native_type(),
                sf.is_list(),
                !sf.is_required(),
                false,
                include_aggregates,
            ))];

            if sf.type_identifier() != TypeIdentifier::Json {
                types.push(map_scalar_input_type_for_field(ctx, &sf)); // Scalar equality shorthand
            }

            types
        }
    }
}

/// Builds shorthand relation equality (`is`) filter for to-one: `where: { relation_field: { ... } }` (no `is` in between).
fn to_one_relation_filter_shorthand_types<'a>(ctx: &'a QuerySchema, rf: &RelationFieldRef) -> InputType<'a> {
    let related_model = rf.related_model();
    let related_input_type = filter_objects::where_object_type(ctx, related_model.into());

    InputType::object(related_input_type)
}

fn to_many_relation_filter_object(ctx: &'_ QuerySchema, rf: RelationFieldRef) -> InputObjectType<'_> {
    let ident = Identifier::new_prisma(IdentifierType::ToManyRelationFilterInput(rf.related_model()));

    let mut object = init_input_object_type(ident);
    object.set_container(rf.related_model());
    object.set_tag(ObjectTag::RelationEnvelope);

    object.set_fields(move || {
        let related_input_type = filter_objects::where_object_type(ctx, rf.related_model().into());
        vec![
            simple_input_field(filters::EVERY, InputType::object(related_input_type.clone()), None).optional(),
            simple_input_field(filters::SOME, InputType::object(related_input_type.clone()), None).optional(),
            simple_input_field(filters::NONE, InputType::object(related_input_type), None).optional(),
        ]
    });
    object
}

fn to_one_relation_filter_object(ctx: &'_ QuerySchema, rf: RelationFieldRef) -> InputObjectType<'_> {
    let ident = Identifier::new_prisma(IdentifierType::ToOneRelationFilterInput(rf.related_model(), rf.arity()));

    let mut object = init_input_object_type(ident);
    object.set_container(rf.related_model());
    object.set_tag(ObjectTag::RelationEnvelope);
    object.set_fields(move || {
        let related_input_type = filter_objects::where_object_type(ctx, rf.related_model().into());

        vec![
            simple_input_field(filters::IS, InputType::object(related_input_type.clone()), None)
                .optional()
                .nullable_if(!rf.is_required()),
            simple_input_field(filters::IS_NOT, InputType::object(related_input_type), None)
                .optional()
                .nullable_if(!rf.is_required()),
        ]
    });

    object
}

/// Builds shorthand composite equality (`equals`) filter for to-one: `where: { composite_field: { ... } }` (no `equals` in between).
fn to_one_composite_filter_shorthand_types(ctx: &'_ QuerySchema, cf: CompositeFieldRef) -> InputType<'_> {
    InputType::object(filter_objects::composite_equality_object(ctx, cf))
}

fn to_one_composite_filter_object(ctx: &'_ QuerySchema, cf: CompositeFieldRef) -> InputObjectType<'_> {
    let ident = Identifier::new_prisma(IdentifierType::ToOneCompositeFilterInput(cf.typ(), cf.arity()));

    let mut object = init_input_object_type(ident);
    object.require_exactly_one_field();
    object.set_container(cf.typ());
    object.set_tag(ObjectTag::CompositeEnvelope);

    object.set_fields(move || {
        let composite_where_object = filter_objects::where_object_type(ctx, cf.typ().into());
        let composite_equals_object = filter_objects::composite_equality_object(ctx, cf.clone());

        let mut fields = vec![
            simple_input_field(filters::EQUALS, InputType::object(composite_equals_object), None)
                .optional()
                .nullable_if(!cf.is_required()),
            simple_input_field(filters::IS, InputType::object(composite_where_object.clone()), None)
                .optional()
                .nullable_if(!cf.is_required()),
            simple_input_field(filters::IS_NOT, InputType::object(composite_where_object), None)
                .optional()
                .nullable_if(!cf.is_required()),
        ];

        if ctx.has_capability(ConnectorCapability::UndefinedType) && cf.is_optional() {
            fields.push(is_set_input_field());
        }

        fields
    });
    object
}

fn to_many_composite_filter_object(ctx: &'_ QuerySchema, cf: CompositeFieldRef) -> InputObjectType<'_> {
    let ident = Identifier::new_prisma(IdentifierType::ToManyCompositeFilterInput(cf.typ()));

    let mut object = init_input_object_type(ident);
    object.require_exactly_one_field();
    object.set_container(cf.typ());
    object.set_tag(ObjectTag::CompositeEnvelope);
    object.set_fields(move || {
        let composite_where_object = filter_objects::where_object_type(ctx, cf.typ().into());
        let composite_equals_object = filter_objects::composite_equality_object(ctx, cf.clone());

        let mut fields = vec![
            simple_input_field(
                filters::EQUALS,
                InputType::list(InputType::object(composite_equals_object)),
                None,
            )
            .optional(),
            simple_input_field(filters::EVERY, InputType::object(composite_where_object.clone()), None).optional(),
            simple_input_field(filters::SOME, InputType::object(composite_where_object.clone()), None).optional(),
            simple_input_field(filters::NONE, InputType::object(composite_where_object), None).optional(),
            simple_input_field(filters::IS_EMPTY, InputType::boolean(), None).optional(),
        ];

        // TODO: Remove from required lists once we have optional lists
        if ctx.has_capability(ConnectorCapability::UndefinedType) {
            fields.push(is_set_input_field());
        }

        fields
    });

    object
}

fn scalar_list_filter_type(ctx: &'_ QuerySchema, sf: ScalarFieldRef) -> InputObjectType<'_> {
    let ident = Identifier::new_prisma(IdentifierType::ScalarListFilterInput(
        ctx.internal_data_model.clone().zip(sf.type_identifier()),
        sf.is_required(),
    ));

    let mut object = init_input_object_type(ident);
    object.require_exactly_one_field();
    object.set_container(sf.container());
    object.set_fields(move || {
        let mapped_nonlist_type = map_scalar_input_type(ctx, sf.type_identifier(), false);
        let mapped_list_type = InputType::list(mapped_nonlist_type.clone());
        let mut fields: Vec<_> = equality_filters(mapped_list_type.clone(), !sf.is_required()).collect();

        let mapped_nonlist_type_with_field_ref_input = mapped_nonlist_type.with_field_ref_input();
        fields.push(
            input_field(filters::HAS, mapped_nonlist_type_with_field_ref_input, None)
                .optional()
                .nullable_if(!sf.is_required())
                .parameterizable(),
        );

        let mapped_list_type_with_field_ref_input = mapped_list_type.with_field_ref_input();
        fields.push(
            input_field(filters::HAS_EVERY, mapped_list_type_with_field_ref_input.clone(), None)
                .optional()
                .parameterizable(),
        );
        fields.push(
            input_field(filters::HAS_SOME, mapped_list_type_with_field_ref_input, None)
                .optional()
                .parameterizable(),
        );
        fields.push(simple_input_field(filters::IS_EMPTY, InputType::boolean(), None).optional());
        fields
    });
    object
}

fn full_scalar_filter_type(
    ctx: &'_ QuerySchema,
    typ: TypeIdentifier,
    native_type: Option<NativeTypeInstance>,
    list: bool,
    nullable: bool,
    nested: bool,
    include_aggregates: bool,
) -> InputObjectType<'_> {
    let native_type_name = native_type.as_ref().map(|nt| nt.name());
    let scalar_type_name = ctx.internal_data_model.clone().zip(typ).type_name().into_owned();
    let type_name = ctx.connector.scalar_filter_name(scalar_type_name, native_type_name);
    let ident = Identifier::new_prisma(scalar_filter_name(
        &type_name,
        list,
        nullable,
        nested,
        include_aggregates,
    ));

    let mut object = init_input_object_type(ident);

    object.set_fields(move || {
        let mapped_scalar_type = map_scalar_input_type(ctx, typ, list);
        let mut fields: Vec<_> = match typ {
            TypeIdentifier::String | TypeIdentifier::UUID => equality_filters(mapped_scalar_type.clone(), nullable)
                .chain(inclusion_filters(ctx, mapped_scalar_type.clone(), nullable))
                .chain(alphanumeric_filters(ctx, mapped_scalar_type.clone()))
                .chain(string_filters(ctx, type_name.as_ref(), mapped_scalar_type.clone()))
                .chain(query_mode_field(ctx, nested))
                .collect(),

            TypeIdentifier::Int
            | TypeIdentifier::BigInt
            | TypeIdentifier::Float
            | TypeIdentifier::DateTime
            | TypeIdentifier::Decimal => equality_filters(mapped_scalar_type.clone(), nullable)
                .chain(inclusion_filters(ctx, mapped_scalar_type.clone(), nullable))
                .chain(alphanumeric_filters(ctx, mapped_scalar_type.clone()))
                .collect(),

            TypeIdentifier::Json => {
                let mut filters: Vec<InputField<'_>> =
                    json_equality_filters(ctx, mapped_scalar_type.clone(), nullable).collect();

                if ctx.supports_any(&[
                    ConnectorCapability::JsonFilteringJsonPath,
                    ConnectorCapability::JsonFilteringArrayPath,
                ]) {
                    filters.extend(json_filters(ctx));

                    if ctx.has_capability(ConnectorCapability::JsonFilteringAlphanumeric) {
                        filters.extend(alphanumeric_filters(ctx, mapped_scalar_type.clone()))
                    }
                }

                filters
            }

            TypeIdentifier::Boolean => equality_filters(mapped_scalar_type.clone(), nullable).collect(),

            TypeIdentifier::Bytes | TypeIdentifier::Enum(_) => equality_filters(mapped_scalar_type.clone(), nullable)
                .chain(inclusion_filters(ctx, mapped_scalar_type.clone(), nullable))
                .collect(),

            TypeIdentifier::Extension(_) => unreachable!("No extension field should reach this path"),

            TypeIdentifier::Unsupported => unreachable!("No unsupported field should reach this path"),
        };

        fields.push(not_filter_field(
            ctx,
            typ,
            native_type.clone(),
            mapped_scalar_type,
            nullable,
            include_aggregates,
            list,
        ));

        if include_aggregates {
            fields.push(aggregate_filter_field(
                ctx,
                aggregations::UNDERSCORE_COUNT,
                TypeIdentifier::Int,
                nullable,
                list,
            ));

            if typ.is_numeric() {
                let avg_type = map_avg_type_ident(typ);
                fields.push(aggregate_filter_field(
                    ctx,
                    aggregations::UNDERSCORE_AVG,
                    avg_type,
                    nullable,
                    list,
                ));

                fields.push(aggregate_filter_field(
                    ctx,
                    aggregations::UNDERSCORE_SUM,
                    typ,
                    nullable,
                    list,
                ));
            }

            if !list {
                fields.push(aggregate_filter_field(
                    ctx,
                    aggregations::UNDERSCORE_MIN,
                    typ,
                    nullable,
                    list,
                ));

                fields.push(aggregate_filter_field(
                    ctx,
                    aggregations::UNDERSCORE_MAX,
                    typ,
                    nullable,
                    list,
                ));
            }
        }

        if ctx.has_capability(ConnectorCapability::UndefinedType) && (list || nullable) {
            fields.push(is_set_input_field());
        }

        fields
    });
    object
}

fn is_set_input_field<'a>() -> InputField<'a> {
    simple_input_field(filters::IS_SET, InputType::boolean(), None).optional()
}

fn equality_filters(mapped_type: InputType<'_>, nullable: bool) -> impl Iterator<Item = InputField<'_>> {
    let types = mapped_type.with_field_ref_input();

    std::iter::once(
        input_field(filters::EQUALS, types, None)
            .optional()
            .nullable_if(nullable)
            .parameterizable(),
    )
}

fn json_equality_filters<'a>(
    ctx: &'a QuerySchema,
    mapped_type: InputType<'a>,
    nullable: bool,
) -> impl Iterator<Item = InputField<'a>> {
    let field = if ctx.has_capability(ConnectorCapability::AdvancedJsonNullability) {
        let enum_type = json_null_filter_enum();
        let mut field_types = mapped_type.with_field_ref_input();
        field_types.push(InputType::Enum(enum_type));

        input_field(filters::EQUALS, field_types, None)
            .optional()
            .parameterizable()
    } else {
        let inner = mapped_type.with_field_ref_input();
        input_field(filters::EQUALS, inner, None)
            .optional()
            .nullable_if(nullable)
            .parameterizable()
    };

    std::iter::once(field)
}

fn inclusion_filters<'a>(
    ctx: &'a QuerySchema,
    mapped_type: InputType<'a>,
    nullable: bool,
) -> impl Iterator<Item = InputField<'a>> {
    let input_type = InputType::list(mapped_type);

    let field_types: Vec<InputType<'_>> = if ctx.has_capability(ConnectorCapability::ScalarLists) {
        input_type.with_field_ref_input()
    } else {
        vec![input_type]
    };

    vec![
        input_field(filters::IN, field_types.clone(), None)
            .optional()
            .nullable_if(nullable)
            .parameterizable(),
        input_field(filters::NOT_IN, field_types, None)
            .optional()
            .nullable_if(nullable)
            .parameterizable(),
    ]
    .into_iter()
}

fn alphanumeric_filters<'a>(ctx: &'a QuerySchema, mapped_type: InputType<'a>) -> impl Iterator<Item = InputField<'a>> {
    // We disable referencing fields on alphanumeric json filters for MySQL & MariaDB because we can't make it work
    // for both database without splitting them into their own connectors.
    let field_types =
        if !mapped_type.is_json() || ctx.has_capability(ConnectorCapability::JsonFilteringAlphanumericFieldRef) {
            mapped_type.with_field_ref_input()
        } else {
            vec![mapped_type]
        };

    vec![
        input_field(filters::LOWER_THAN, field_types.clone(), None)
            .optional()
            .parameterizable(),
        input_field(filters::LOWER_THAN_OR_EQUAL, field_types.clone(), None)
            .optional()
            .parameterizable(),
        input_field(filters::GREATER_THAN, field_types.clone(), None)
            .optional()
            .parameterizable(),
        input_field(filters::GREATER_THAN_OR_EQUAL, field_types, None)
            .optional()
            .parameterizable(),
    ]
    .into_iter()
}

fn string_filters<'a>(
    ctx: &'a QuerySchema,
    input_object_type_name: &str,
    mapped_type: InputType<'a>,
) -> impl Iterator<Item = InputField<'a>> {
    let field_types = mapped_type.clone().with_field_ref_input();

    let string_filters = ctx.connector.string_filters(input_object_type_name);
    let mut string_filters: Vec<_> = string_filters
        .iter()
        .map(|filter| {
            input_field(filter.name(), field_types.clone(), None)
                .optional()
                .parameterizable()
        })
        .collect();

    if ctx.can_full_text_search() {
        string_filters.push(
            simple_input_field(filters::SEARCH, mapped_type, None)
                .optional()
                .parameterizable(),
        );
    }

    string_filters.into_iter()
}

fn json_filters(ctx: &'_ QuerySchema) -> impl Iterator<Item = InputField<'_>> {
    // TODO: also add json-specific "keys" filters
    // TODO: add json_type filter
    let path_type = if ctx.has_capability(ConnectorCapability::JsonFilteringJsonPath) {
        InputType::string()
    } else if ctx.has_capability(ConnectorCapability::JsonFilteringArrayPath) {
        InputType::list(InputType::string())
    } else {
        unreachable!()
    };
    let string_with_field_ref_input = InputType::string().with_field_ref_input();
    let json_with_field_ref_input = InputType::json().with_field_ref_input();
    let mode_enum_type = InputType::enum_type(query_mode_enum()).with_field_ref_input();

    let mut base = vec![
        simple_input_field(filters::PATH, path_type, None).optional(),
        input_field(
            filters::MODE,
            mode_enum_type,
            Some(DefaultKind::Single(PrismaValue::Enum(filters::DEFAULT.to_owned()))),
        )
        .optional(),
        input_field(filters::STRING_CONTAINS, string_with_field_ref_input.clone(), None)
            .optional()
            .parameterizable(),
        input_field(filters::STRING_STARTS_WITH, string_with_field_ref_input.clone(), None)
            .optional()
            .parameterizable(),
        input_field(filters::STRING_ENDS_WITH, string_with_field_ref_input, None)
            .optional()
            .parameterizable(),
        input_field(filters::ARRAY_STARTS_WITH, json_with_field_ref_input.clone(), None)
            .optional()
            .nullable()
            .parameterizable(),
        input_field(filters::ARRAY_ENDS_WITH, json_with_field_ref_input.clone(), None)
            .optional()
            .nullable()
            .parameterizable(),
    ];

    if ctx.has_capability(ConnectorCapability::JsonArrayContains) {
        base.push(
            input_field(filters::ARRAY_CONTAINS, json_with_field_ref_input.clone(), None)
                .optional()
                .nullable()
                .parameterizable(),
        )
    }

    base.into_iter()
}

fn query_mode_field(ctx: &'_ QuerySchema, nested: bool) -> impl Iterator<Item = InputField<'_>> {
    // Limit query mode field to the topmost filter level.
    // Only build mode field for connectors with insensitive filter support.
    let fields = if !nested && ctx.has_capability(ConnectorCapability::InsensitiveFilters) {
        let enum_type = query_mode_enum();

        let field = simple_input_field(
            filters::MODE,
            InputType::enum_type(enum_type),
            Some(DefaultKind::Single(PrismaValue::Enum(filters::DEFAULT.to_owned()))),
        )
        .optional();

        vec![field]
    } else {
        vec![]
    };

    fields.into_iter()
}

fn aggregate_filter_field(
    ctx: &'_ QuerySchema,
    aggregation: impl Into<std::borrow::Cow<'static, str>>,
    typ: TypeIdentifier,
    nullable: bool,
    list: bool,
) -> InputField<'_> {
    let filters = full_scalar_filter_type(ctx, typ, None, list, nullable, true, false);
    simple_input_field(aggregation.into(), InputType::object(filters), None).optional()
}

fn map_avg_type_ident(typ: TypeIdentifier) -> TypeIdentifier {
    match &typ {
        TypeIdentifier::Int | TypeIdentifier::BigInt | TypeIdentifier::Float => TypeIdentifier::Float,
        _ => typ,
    }
}

// Shorthand `not equals` filter input field, skips the nested object filter.
fn not_filter_field<'a>(
    ctx: &'a QuerySchema,
    typ: TypeIdentifier,
    native_type: Option<NativeTypeInstance>,
    mapped_scalar_type: InputType<'a>,
    is_nullable: bool,
    include_aggregates: bool,
    is_list: bool,
) -> InputField<'a> {
    let has_adv_json = ctx.has_capability(ConnectorCapability::AdvancedJsonNullability);

    match typ {
        // Json is not nullable on dbs with `AdvancedJsonNullability`, only by proxy through an enum.
        TypeIdentifier::Json if has_adv_json => {
            let enum_type = json_null_filter_enum();
            let mut field_types = mapped_scalar_type.with_field_ref_input();
            field_types.push(InputType::Enum(enum_type));

            input_field(filters::NOT_LOWERCASE, field_types, None)
                .optional()
                .parameterizable()
        }

        TypeIdentifier::Json => {
            let ty = mapped_scalar_type.with_field_ref_input();
            input_field(filters::NOT_LOWERCASE, ty, None)
                .optional()
                .nullable_if(is_nullable)
                .parameterizable()
        }

        _ => {
            // Full nested filter. Only available on non-JSON fields.
            let shorthand = InputType::object(full_scalar_filter_type(
                ctx,
                typ,
                native_type,
                is_list,
                is_nullable,
                true,
                include_aggregates,
            ));

            input_field(filters::NOT_LOWERCASE, vec![mapped_scalar_type, shorthand], None)
                .optional()
                .nullable_if(is_nullable)
                .parameterizable()
        }
    }
}

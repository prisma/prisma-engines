use super::objects::*;
use super::*;
use constants::{aggregations, filters};
use datamodel_connector::ConnectorCapability;
use prisma_models::{dml::DefaultValue, CompositeFieldRef, PrismaValue};

/// Builds filter types for the given model field.
pub(crate) fn get_field_filter_types(
    ctx: &mut BuilderContext,
    field: &ModelField,
    include_aggregates: bool,
) -> Vec<InputType> {
    match field {
        ModelField::Relation(rf) if rf.is_list() => {
            vec![InputType::object(to_many_relation_filter_object(ctx, rf))]
        }

        ModelField::Relation(rf) => {
            vec![
                InputType::object(to_one_relation_filter_object(ctx, rf)),
                to_one_relation_filter_shorthand_types(ctx, rf),
            ]
        }

        ModelField::Composite(cf) if cf.is_list() => vec![
            InputType::object(to_many_composite_filter_object(ctx, cf)),
            InputType::list(to_one_composite_filter_shorthand_types(ctx, cf)),
        ],

        ModelField::Composite(cf) => vec![
            InputType::object(to_one_composite_filter_object(ctx, cf)),
            to_one_composite_filter_shorthand_types(ctx, cf),
        ],

        ModelField::Scalar(sf) if field.is_list() => vec![InputType::object(scalar_list_filter_type(ctx, sf))],

        ModelField::Scalar(sf) => {
            let mut types = vec![InputType::object(full_scalar_filter_type(
                ctx,
                &sf.type_identifier,
                sf.is_list(),
                !sf.is_required(),
                false,
                include_aggregates,
            ))];

            if sf.type_identifier != TypeIdentifier::Json {
                types.push(map_scalar_input_type_for_field(ctx, sf)); // Scalar equality shorthand
            }

            types
        }
    }
}

/// Builds shorthand relation equality (`is`) filter for to-one: `where: { relation_field: { ... } }` (no `is` in between).
fn to_one_relation_filter_shorthand_types(ctx: &mut BuilderContext, rf: &RelationFieldRef) -> InputType {
    let related_model = rf.related_model();
    let related_input_type = filter_objects::where_object_type(ctx, &related_model);

    InputType::object(related_input_type)
}

fn to_many_relation_filter_object(ctx: &mut BuilderContext, rf: &RelationFieldRef) -> InputObjectTypeWeakRef {
    let related_model = rf.related_model();
    let related_input_type = filter_objects::where_object_type(ctx, &related_model);
    let ident = Identifier::new(
        format!("{}ListRelationFilter", capitalize(&related_model.name)),
        PRISMA_NAMESPACE,
    );

    return_cached_input!(ctx, &ident);
    let object = Arc::new(init_input_object_type(ident.clone()));
    ctx.cache_input_type(ident, object.clone());

    let fields = vec![
        input_field(filters::EVERY, InputType::object(related_input_type.clone()), None).optional(),
        input_field(filters::SOME, InputType::object(related_input_type.clone()), None).optional(),
        input_field(filters::NONE, InputType::object(related_input_type), None).optional(),
    ];

    object.set_fields(fields);
    Arc::downgrade(&object)
}

fn to_one_relation_filter_object(ctx: &mut BuilderContext, rf: &RelationFieldRef) -> InputObjectTypeWeakRef {
    let related_model = rf.related_model();
    let related_input_type = filter_objects::where_object_type(ctx, &related_model);
    let ident = Identifier::new(
        format!("{}RelationFilter", capitalize(&related_model.name)),
        PRISMA_NAMESPACE,
    );

    return_cached_input!(ctx, &ident);
    let object = Arc::new(init_input_object_type(ident.clone()));
    ctx.cache_input_type(ident, object.clone());

    let fields = vec![
        input_field(filters::IS, InputType::object(related_input_type.clone()), None)
            .optional()
            .nullable_if(!rf.is_required()),
        input_field(filters::IS_NOT, InputType::object(related_input_type), None)
            .optional()
            .nullable_if(!rf.is_required()),
    ];

    object.set_fields(fields);
    Arc::downgrade(&object)
}

/// Builds shorthand composite equality (`equals`) filter for to-one: `where: { composite_field: { ... } }` (no `equals` in between).
fn to_one_composite_filter_shorthand_types(ctx: &mut BuilderContext, cf: &CompositeFieldRef) -> InputType {
    let equality_object_type = filter_objects::composite_equality_object(ctx, cf);

    InputType::object(equality_object_type)
}

fn to_one_composite_filter_object(ctx: &mut BuilderContext, cf: &CompositeFieldRef) -> InputObjectTypeWeakRef {
    let nullable = if cf.is_optional() { "Nullable" } else { "" };
    let ident = Identifier::new(
        format!("{}{}CompositeFilter", capitalize(&cf.typ.name), nullable),
        PRISMA_NAMESPACE,
    );
    return_cached_input!(ctx, &ident);

    let mut object = init_input_object_type(ident.clone());
    object.require_exactly_one_field();
    object.set_tag(ObjectTag::CompositeEnvelope);

    let object = Arc::new(object);

    ctx.cache_input_type(ident, object.clone());

    let composite_where_object = filter_objects::where_object_type(ctx, &cf.typ);
    let composite_equals_object = filter_objects::composite_equality_object(ctx, cf);

    let mut fields = vec![
        input_field(filters::EQUALS, InputType::object(composite_equals_object), None)
            .optional()
            .nullable_if(!cf.is_required()),
        input_field(filters::IS, InputType::object(composite_where_object.clone()), None)
            .optional()
            .nullable_if(!cf.is_required()),
        input_field(filters::IS_NOT, InputType::object(composite_where_object), None)
            .optional()
            .nullable_if(!cf.is_required()),
    ];

    if ctx.has_capability(ConnectorCapability::UndefinedType) && cf.is_optional() {
        fields.push(is_set_input_field());
    }

    object.set_fields(fields);
    Arc::downgrade(&object)
}

fn to_many_composite_filter_object(ctx: &mut BuilderContext, cf: &CompositeFieldRef) -> InputObjectTypeWeakRef {
    let ident = Identifier::new(
        format!("{}CompositeListFilter", capitalize(&cf.typ.name)),
        PRISMA_NAMESPACE,
    );
    return_cached_input!(ctx, &ident);

    let mut object = init_input_object_type(ident.clone());
    object.require_exactly_one_field();
    object.set_tag(ObjectTag::CompositeEnvelope);

    let object = Arc::new(object);
    ctx.cache_input_type(ident, object.clone());

    let composite_where_object = filter_objects::where_object_type(ctx, &cf.typ);
    let composite_equals_object = filter_objects::composite_equality_object(ctx, cf);

    let mut fields = vec![
        input_field(
            filters::EQUALS,
            InputType::list(InputType::object(composite_equals_object)),
            None,
        )
        .optional(),
        input_field(filters::EVERY, InputType::object(composite_where_object.clone()), None).optional(),
        input_field(filters::SOME, InputType::object(composite_where_object.clone()), None).optional(),
        input_field(filters::NONE, InputType::object(composite_where_object), None).optional(),
        input_field(filters::IS_EMPTY, InputType::boolean(), None).optional(),
    ];

    // TODO: Remove from required lists once we have optional lists
    if ctx.has_capability(ConnectorCapability::UndefinedType) {
        fields.push(is_set_input_field());
    }

    object.set_fields(fields);
    Arc::downgrade(&object)
}

fn scalar_list_filter_type(ctx: &mut BuilderContext, sf: &ScalarFieldRef) -> InputObjectTypeWeakRef {
    let ident = Identifier::new(
        scalar_filter_name(&sf.type_identifier, true, !sf.is_required(), false, false),
        PRISMA_NAMESPACE,
    );
    return_cached_input!(ctx, &ident);

    let mut object = init_input_object_type(ident.clone());
    object.require_exactly_one_field();

    let object = Arc::new(object);
    ctx.cache_input_type(ident, object.clone());

    let mapped_nonlist_type = map_scalar_input_type(ctx, &sf.type_identifier, false);
    let mapped_list_type = InputType::list(mapped_nonlist_type.clone());
    let mut fields: Vec<_> = equality_filters(ctx, mapped_list_type.clone(), !sf.is_required()).collect();

    fields.push(
        input_field(
            filters::HAS,
            vec![mapped_nonlist_type, InputType::object(field_reference_object_type(ctx))],
            None,
        )
        .optional()
        .nullable_if(!sf.is_required()),
    );

    fields.push(
        input_field(
            filters::HAS_EVERY,
            vec![
                mapped_list_type.clone(),
                InputType::object(field_reference_object_type(ctx)),
            ],
            None,
        )
        .optional(),
    );
    fields.push(
        input_field(
            filters::HAS_SOME,
            vec![mapped_list_type, InputType::object(field_reference_object_type(ctx))],
            None,
        )
        .optional(),
    );
    fields.push(input_field(filters::IS_EMPTY, InputType::boolean(), None).optional());

    object.set_fields(fields);
    Arc::downgrade(&object)
}

fn full_scalar_filter_type(
    ctx: &mut BuilderContext,
    typ: &TypeIdentifier,
    list: bool,
    nullable: bool,
    nested: bool,
    include_aggregates: bool,
) -> InputObjectTypeWeakRef {
    let ident = Identifier::new(
        scalar_filter_name(typ, list, nullable, nested, include_aggregates),
        PRISMA_NAMESPACE,
    );

    return_cached_input!(ctx, &ident);

    let object = Arc::new(init_input_object_type(ident.clone()));
    ctx.cache_input_type(ident, object.clone());

    let mapped_scalar_type = map_scalar_input_type(ctx, typ, list);

    let mut fields: Vec<_> = match typ {
        TypeIdentifier::String | TypeIdentifier::UUID => equality_filters(ctx, mapped_scalar_type.clone(), nullable)
            .chain(inclusion_filters(ctx, mapped_scalar_type.clone(), nullable))
            .chain(alphanumeric_filters(ctx, mapped_scalar_type.clone()))
            .chain(string_filters(ctx, mapped_scalar_type.clone()))
            .chain(query_mode_field(ctx, nested))
            .collect(),

        TypeIdentifier::Int
        | TypeIdentifier::BigInt
        | TypeIdentifier::Float
        | TypeIdentifier::DateTime
        | TypeIdentifier::Decimal => equality_filters(ctx, mapped_scalar_type.clone(), nullable)
            .chain(inclusion_filters(ctx, mapped_scalar_type.clone(), nullable))
            .chain(alphanumeric_filters(ctx, mapped_scalar_type.clone()))
            .collect(),

        TypeIdentifier::Json => {
            let mut filters: Vec<InputField> =
                json_equality_filters(ctx, mapped_scalar_type.clone(), nullable).collect();

            if ctx.capabilities.supports_any(&[
                ConnectorCapability::JsonFilteringJsonPath,
                ConnectorCapability::JsonFilteringArrayPath,
            ]) {
                filters.extend(json_filters(ctx));

                if ctx
                    .capabilities
                    .contains(ConnectorCapability::JsonFilteringAlphanumeric)
                {
                    filters.extend(alphanumeric_filters(ctx, mapped_scalar_type.clone()))
                }
            }

            filters
        }

        TypeIdentifier::Boolean | TypeIdentifier::Xml => {
            equality_filters(ctx, mapped_scalar_type.clone(), nullable).collect()
        }

        TypeIdentifier::Bytes | TypeIdentifier::Enum(_) => equality_filters(ctx, mapped_scalar_type.clone(), nullable)
            .chain(inclusion_filters(ctx, mapped_scalar_type.clone(), nullable))
            .collect(),

        TypeIdentifier::Unsupported => unreachable!("No unsupported field should reach that path"),
    };

    fields.push(not_filter_field(
        ctx,
        typ,
        mapped_scalar_type,
        nullable,
        include_aggregates,
        list,
    ));

    if include_aggregates {
        fields.push(aggregate_filter_field(
            ctx,
            aggregations::UNDERSCORE_COUNT,
            &TypeIdentifier::Int,
            nullable,
            list,
        ));

        if typ.is_numeric() {
            let avg_type = map_avg_type_ident(typ.clone());
            fields.push(aggregate_filter_field(
                ctx,
                aggregations::UNDERSCORE_AVG,
                &avg_type,
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

    object.set_fields(fields);
    Arc::downgrade(&object)
}

fn is_set_input_field() -> InputField {
    input_field(filters::IS_SET, InputType::boolean(), None).optional()
}

fn equality_filters(
    ctx: &mut BuilderContext,
    mapped_type: InputType,
    nullable: bool,
) -> impl Iterator<Item = InputField> {
    std::iter::once(
        input_field(filters::EQUALS, mapped_type.with_field_ref_input(ctx), None)
            .optional()
            .nullable_if(nullable),
    )
}

fn json_equality_filters(
    ctx: &mut BuilderContext,
    mapped_type: InputType,
    nullable: bool,
) -> impl Iterator<Item = InputField> {
    let field = if ctx.has_capability(ConnectorCapability::AdvancedJsonNullability) {
        let enum_type = json_null_filter_enum(ctx);
        let mut field_types = mapped_type.with_field_ref_input(ctx);
        field_types.push(InputType::Enum(enum_type));

        input_field(filters::EQUALS, field_types, None).optional()
    } else {
        input_field(filters::EQUALS, mapped_type.with_field_ref_input(ctx), None)
            .optional()
            .nullable_if(nullable)
    };

    std::iter::once(field)
}

fn inclusion_filters(
    ctx: &mut BuilderContext,
    mapped_type: InputType,
    nullable: bool,
) -> impl Iterator<Item = InputField> {
    let typ = InputType::list(mapped_type);
    let field_types: Vec<InputType> =
        typ.with_field_ref_input_if(ctx, ctx.has_capability(ConnectorCapability::ScalarLists));

    vec![
        input_field(filters::IN, field_types.clone(), None)
            .optional()
            .nullable_if(nullable),
        input_field(filters::NOT_IN, field_types, None)
            .optional()
            .nullable_if(nullable), // Kept for legacy reasons!
    ]
    .into_iter()
}

fn alphanumeric_filters(ctx: &mut BuilderContext, mapped_type: InputType) -> impl Iterator<Item = InputField> {
    let field_types = mapped_type.with_field_ref_input(ctx);

    vec![
        input_field(filters::LOWER_THAN, field_types.clone(), None).optional(),
        input_field(filters::LOWER_THAN_OR_EQUAL, field_types.clone(), None).optional(),
        input_field(filters::GREATER_THAN, field_types.clone(), None).optional(),
        input_field(filters::GREATER_THAN_OR_EQUAL, field_types, None).optional(),
    ]
    .into_iter()
}

fn string_filters(ctx: &mut BuilderContext, mapped_type: InputType) -> impl Iterator<Item = InputField> {
    let field_types = mapped_type.clone().with_field_ref_input(ctx);

    let mut string_filters = vec![
        input_field(filters::CONTAINS, field_types.clone(), None).optional(),
        input_field(filters::STARTS_WITH, field_types.clone(), None).optional(),
        input_field(filters::ENDS_WITH, field_types, None).optional(),
    ];

    if ctx.can_full_text_search() {
        string_filters.push(input_field(filters::SEARCH, mapped_type, None).optional());
    }

    string_filters.into_iter()
}

fn json_filters(ctx: &mut BuilderContext) -> impl Iterator<Item = InputField> {
    // TODO: also add json-specific "keys" filters
    // TODO: add json_type filter
    let path_type = if ctx.capabilities.contains(ConnectorCapability::JsonFilteringJsonPath) {
        InputType::string()
    } else if ctx.capabilities.contains(ConnectorCapability::JsonFilteringArrayPath) {
        InputType::list(InputType::string())
    } else {
        unreachable!()
    };

    vec![
        input_field(filters::PATH, path_type.with_field_ref_input(ctx), None).optional(),
        input_field(
            filters::STRING_CONTAINS,
            InputType::string().with_field_ref_input(ctx),
            None,
        )
        .optional(),
        input_field(
            filters::STRING_STARTS_WITH,
            InputType::string().with_field_ref_input(ctx),
            None,
        )
        .optional(),
        input_field(
            filters::STRING_ENDS_WITH,
            InputType::string().with_field_ref_input(ctx),
            None,
        )
        .optional(),
        input_field(
            filters::ARRAY_CONTAINS,
            InputType::json().with_field_ref_input(ctx),
            None,
        )
        .optional()
        .nullable(),
        input_field(
            filters::ARRAY_STARTS_WITH,
            InputType::json().with_field_ref_input(ctx),
            None,
        )
        .optional()
        .nullable(),
        input_field(
            filters::ARRAY_ENDS_WITH,
            InputType::json().with_field_ref_input(ctx),
            None,
        )
        .optional()
        .nullable(),
    ]
    .into_iter()
}

fn query_mode_field(ctx: &mut BuilderContext, nested: bool) -> impl Iterator<Item = InputField> {
    // Limit query mode field to the topmost filter level.
    // Only build mode field for connectors with insensitive filter support.
    let fields = if !nested && ctx.capabilities.contains(ConnectorCapability::InsensitiveFilters) {
        let enum_type = query_mode_enum(ctx);

        let field = input_field(
            filters::MODE,
            InputType::enum_type(enum_type),
            Some(DefaultValue::new_single(PrismaValue::Enum(filters::DEFAULT.to_owned()))),
        )
        .optional();

        vec![field]
    } else {
        vec![]
    };

    fields.into_iter()
}

fn scalar_filter_name(
    typ: &TypeIdentifier,
    list: bool,
    nullable: bool,
    nested: bool,
    include_aggregates: bool,
) -> String {
    let list = if list { "List" } else { "" };
    let nullable = if nullable { "Nullable" } else { "" };
    let nested = if nested { "Nested" } else { "" };
    let aggregates = if include_aggregates { "WithAggregates" } else { "" };

    match typ {
        TypeIdentifier::UUID => format!("{}Uuid{}{}{}Filter", nested, nullable, list, aggregates),
        TypeIdentifier::String => format!("{}String{}{}{}Filter", nested, nullable, list, aggregates),
        TypeIdentifier::Int => format!("{}Int{}{}{}Filter", nested, nullable, list, aggregates),
        TypeIdentifier::BigInt => format!("{}BigInt{}{}{}Filter", nested, nullable, list, aggregates),
        TypeIdentifier::Float => format!("{}Float{}{}{}Filter", nested, nullable, list, aggregates),
        TypeIdentifier::Decimal => format!("{}Decimal{}{}{}Filter", nested, nullable, list, aggregates),
        TypeIdentifier::Boolean => format!("{}Bool{}{}{}Filter", nested, nullable, list, aggregates),
        TypeIdentifier::DateTime => format!("{}DateTime{}{}{}Filter", nested, nullable, list, aggregates),
        TypeIdentifier::Json => format!("{}Json{}{}{}Filter", nested, nullable, list, aggregates),
        TypeIdentifier::Enum(ref e) => format!("{}Enum{}{}{}{}Filter", nested, e, nullable, list, aggregates),
        TypeIdentifier::Xml => format!("{}Xml{}{}{}Filter", nested, nullable, list, aggregates),
        TypeIdentifier::Bytes => format!("{}Bytes{}{}{}Filter", nested, nullable, list, aggregates),
        TypeIdentifier::Unsupported => unreachable!("No unsupported field should reach that path"),
    }
}

fn aggregate_filter_field(
    ctx: &mut BuilderContext,
    aggregation: &str,
    typ: &TypeIdentifier,
    nullable: bool,
    list: bool,
) -> InputField {
    let filters = full_scalar_filter_type(ctx, typ, list, nullable, true, false);
    input_field(aggregation, InputType::object(filters), None).optional()
}

fn map_avg_type_ident(typ: TypeIdentifier) -> TypeIdentifier {
    match &typ {
        TypeIdentifier::Int | TypeIdentifier::BigInt | TypeIdentifier::Float => TypeIdentifier::Float,
        _ => typ,
    }
}

// Shorthand `not equals` filter input field, skips the nested object filter.
fn not_filter_field(
    ctx: &mut BuilderContext,
    typ: &TypeIdentifier,
    mapped_scalar_type: InputType,
    is_nullable: bool,
    include_aggregates: bool,
    is_list: bool,
) -> InputField {
    let has_adv_json = ctx.has_capability(ConnectorCapability::AdvancedJsonNullability);

    match typ {
        // Json is not nullable on dbs with `AdvancedJsonNullability`, only by proxy through an enum.
        TypeIdentifier::Json if has_adv_json => {
            let enum_type = json_null_filter_enum(ctx);
            let mut field_types = mapped_scalar_type.with_field_ref_input(ctx);
            field_types.push(InputType::Enum(enum_type));

            input_field(filters::NOT_LOWERCASE, field_types, None).optional()
        }

        TypeIdentifier::Json => input_field(
            filters::NOT_LOWERCASE,
            mapped_scalar_type.with_field_ref_input(ctx),
            None,
        )
        .optional()
        .nullable_if(is_nullable),

        _ => {
            // Full nested filter. Only available on non-JSON fields.
            let shorthand = InputType::object(full_scalar_filter_type(
                ctx,
                typ,
                is_list,
                is_nullable,
                true,
                include_aggregates,
            ));

            input_field(filters::NOT_LOWERCASE, vec![mapped_scalar_type, shorthand], None)
                .optional()
                .nullable_if(is_nullable)
        }
    }
}

fn field_reference_object_type(ctx: &mut BuilderContext) -> InputObjectTypeWeakRef {
    let ident = Identifier::new("FieldRefInput", PRISMA_NAMESPACE);

    return_cached_input!(ctx, &ident);

    let mut object = init_input_object_type(ident.clone());
    object.set_tag(ObjectTag::FieldRef);

    let object = Arc::new(object);
    ctx.cache_input_type(ident, object.clone());

    object.set_fields(vec![input_field(filters::REF, InputType::string(), None)]);

    Arc::downgrade(&object)
}

trait WithFieldReferenceInputExtension {
    fn with_field_ref_input(self, ctx: &mut BuilderContext) -> Vec<InputType>;
    fn with_field_ref_input_if(self, ctx: &mut BuilderContext, cond: bool) -> Vec<InputType>;
}

impl WithFieldReferenceInputExtension for InputType {
    fn with_field_ref_input(self, ctx: &mut BuilderContext) -> Vec<InputType> {
        let mut field_types: Vec<InputType> = vec![self];

        if ctx.has_feature(&PreviewFeature::FieldReference) {
            field_types.push(InputType::object(field_reference_object_type(ctx)));
        }

        field_types
    }

    fn with_field_ref_input_if(self, ctx: &mut BuilderContext, cond: bool) -> Vec<InputType> {
        if cond {
            self.with_field_ref_input(ctx)
        } else {
            vec![self]
        }
    }
}

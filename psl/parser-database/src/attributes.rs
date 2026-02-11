mod default;
mod id;
mod map;
mod native_types;
mod schema;
mod shard_key;

use crate::{
    DatamodelError, ScalarFieldId, ScalarType, StringId,
    ast::{self, WithName, WithSpan},
    coerce, coerce_array,
    context::Context,
    types::{
        CompositeTypeField, EnumAttributes, FieldWithArgs, IndexAlgorithm, IndexAttribute, IndexFieldPath, IndexType,
        ModelAttributes, OperatorClassStore, RelationField, ScalarField, ScalarFieldType, SortOrder, WhereClause,
        WhereCondition, WhereFieldCondition, WhereValue,
    },
    walkers::RelationFieldId,
};
use diagnostics::Span;
use itertools::Itertools;
use std::{borrow::Cow, cell::Cell, fmt::Display};

pub(super) fn resolve_attributes(ctx: &mut Context<'_>) {
    for rfid in ctx.types.iter_relation_field_ids() {
        visit_relation_field_attributes(rfid, ctx);
    }

    for top in ctx.iter_tops() {
        match top {
            ((file_id, ast::TopId::Model(model_id)), ast::Top::Model(_)) => {
                resolve_model_attributes((file_id, model_id), ctx)
            }
            ((file_id, ast::TopId::Enum(enum_id)), ast::Top::Enum(ast_enum)) => {
                resolve_enum_attributes((file_id, enum_id), ast_enum, ctx)
            }
            ((file_id, ast::TopId::CompositeType(ctid)), ast::Top::CompositeType(ct)) => {
                resolve_composite_type_attributes((file_id, ctid), ct, ctx)
            }
            _ => (),
        }
    }
}

fn resolve_composite_type_attributes<'db>(
    ctid: crate::CompositeTypeId,
    ct: &'db ast::CompositeType,
    ctx: &mut Context<'db>,
) {
    for (field_id, field) in ct.iter_fields() {
        let CompositeTypeField { r#type, .. } =
            if let Some(val) = ctx.types.composite_type_fields.get(&(ctid, field_id)) {
                val.clone()
            } else {
                continue;
            };

        ctx.visit_attributes((ctid.0, (ctid.1, field_id)));

        if let ScalarFieldType::BuiltInScalar(_scalar_type) = r#type {
            // native type attributes
            if let Some((datasource_name, type_name, args)) = ctx.visit_datasource_scoped() {
                native_types::visit_composite_type_field_native_type_attribute(
                    (ctid, field_id),
                    datasource_name,
                    type_name,
                    &ctx.asts[args],
                    ctx,
                )
            }
        }

        // @map
        if ctx.visit_optional_single_attr("map") {
            map::composite_type_field(ct, field, ctid, field_id, ctx);
            ctx.validate_visited_arguments();
        }

        // @default
        if ctx.visit_optional_single_attr("default") {
            default::visit_composite_field_default(ctid, field_id, r#type, ctx);
            ctx.validate_visited_arguments();
        }

        ctx.validate_visited_attributes();
    }
}

fn resolve_enum_attributes<'db>(enum_id: crate::EnumId, ast_enum: &'db ast::Enum, ctx: &mut Context<'db>) {
    let mut enum_attributes = EnumAttributes::default();

    for (value_id, _) in ast_enum.iter_values() {
        ctx.visit_attributes((enum_id.0, (enum_id.1, value_id)));
        // @map
        if ctx.visit_optional_single_attr("map") {
            if let Some(mapped_name) = map::visit_map_attribute(ctx) {
                enum_attributes.mapped_values.insert(value_id, mapped_name);
                ctx.mapped_enum_value_names.insert((enum_id, mapped_name), value_id);
            }
            ctx.validate_visited_arguments();
        }
        ctx.validate_visited_attributes();
    }

    // Now validate the enum attributes.

    ctx.visit_attributes(enum_id);

    // @@map
    if ctx.visit_optional_single_attr("map") {
        if let Some(mapped_name) = map::visit_map_attribute(ctx) {
            enum_attributes.mapped_name = Some(mapped_name);
            ctx.mapped_enum_names.insert(mapped_name, enum_id);
        }
        ctx.validate_visited_arguments();
    }

    // @@schema
    if ctx.visit_optional_single_attr("schema") {
        schema::r#enum(&mut enum_attributes, ctx);
        ctx.validate_visited_arguments();
    }

    ctx.types.enum_attributes.insert(enum_id, enum_attributes);
    ctx.validate_visited_attributes();
}

fn resolve_model_attributes(model_id: crate::ModelId, ctx: &mut Context<'_>) {
    let mut model_attributes = ModelAttributes::default();

    // First resolve all the attributes defined on fields **in isolation**.
    for sfid in ctx.types.range_model_scalar_field_ids(model_id) {
        visit_scalar_field_attributes(sfid, &mut model_attributes, ctx);
    }

    // Resolve all the attributes defined on the model itself **in isolation**.
    ctx.visit_attributes(model_id);

    // @@ignore
    if ctx.visit_optional_single_attr("ignore") {
        visit_model_ignore(model_id, &mut model_attributes, ctx);
        ctx.validate_visited_arguments();
    }

    // @@id
    if ctx.visit_optional_single_attr("id") {
        id::model(&mut model_attributes, model_id, ctx);
        ctx.validate_visited_arguments();
    }

    // @@map
    if ctx.visit_optional_single_attr("map") {
        map::model(&mut model_attributes, ctx);
        ctx.validate_visited_arguments();
    }

    // @@schema
    if ctx.visit_optional_single_attr("schema") {
        schema::model(&mut model_attributes, ctx);
        ctx.validate_visited_arguments();
    }

    // @@index
    while ctx.visit_repeated_attr("index") {
        model_index(&mut model_attributes, model_id, ctx);
        ctx.validate_visited_arguments();
    }

    // @@unique
    while ctx.visit_repeated_attr("unique") {
        model_unique(&mut model_attributes, model_id, ctx);
        ctx.validate_visited_arguments();
    }

    // @@fulltext
    while ctx.visit_repeated_attr("fulltext") {
        model_fulltext(&mut model_attributes, model_id, ctx);
        ctx.validate_visited_arguments();
    }

    // @@shardKey
    if ctx.visit_optional_single_attr("shardKey") {
        shard_key::model(&mut model_attributes, model_id, ctx);
        ctx.validate_visited_arguments();
    }

    // Model-global validations
    id::validate_id_field_arities(model_id, &model_attributes, ctx);
    shard_key::validate_shard_key_field_arities(model_id, &model_attributes, ctx);

    ctx.types.model_attributes.insert(model_id, model_attributes);
    ctx.validate_visited_attributes();
}

fn visit_scalar_field_attributes(
    scalar_field_id: ScalarFieldId,
    model_data: &mut ModelAttributes,
    ctx: &mut Context<'_>,
) {
    let ScalarField {
        model_id,
        field_id,
        r#type,
        ..
    } = ctx.types[scalar_field_id];
    let ast_model = &ctx.asts[model_id];
    let ast_field = &ast_model[field_id];
    ctx.visit_scalar_field_attributes(model_id, field_id);

    // @map
    if ctx.visit_optional_single_attr("map") {
        map::scalar_field(scalar_field_id, ast_model, ast_field, model_id, field_id, ctx);
        ctx.validate_visited_arguments();
    }

    // @ignore
    if ctx.visit_optional_single_attr("ignore") {
        if matches!(r#type, ScalarFieldType::Unsupported(_)) {
            ctx.push_attribute_validation_error("Fields of type `Unsupported` cannot take an `@ignore` attribute. They are already treated as ignored by the client due to their type.");
        } else {
            ctx.types[scalar_field_id].is_ignored = true;
        }
        ctx.validate_visited_arguments();
    }

    // @relation
    if ctx.visit_optional_single_attr("relation") {
        ctx.push_attribute_validation_error("Invalid field type, not a relation.");
        ctx.validate_visited_arguments();
    }

    // @id
    if ctx.visit_optional_single_attr("id") {
        id::field(ast_model, scalar_field_id, field_id, model_data, ctx);
        ctx.validate_visited_arguments();
    }

    // @updatedAt
    if ctx.visit_optional_single_attr("updatedAt") {
        if !matches!(r#type, ScalarFieldType::BuiltInScalar(crate::ScalarType::DateTime)) {
            ctx.push_attribute_validation_error("Fields that are marked with @updatedAt must be of type DateTime.");
        }

        if ast_field.arity.is_list() {
            ctx.push_attribute_validation_error("Fields that are marked with @updatedAt cannot be lists.");
        }

        ctx.types[scalar_field_id].is_updated_at = true;
        ctx.validate_visited_arguments();
    }

    // @default
    if ctx.visit_optional_single_attr("default") {
        default::visit_model_field_default(scalar_field_id, model_id, field_id, r#type, ctx);
        ctx.validate_visited_arguments();
    }

    if let ScalarFieldType::BuiltInScalar(_) | ScalarFieldType::Extension(_) = r#type {
        // native type attributes
        if let Some((datasource_name, type_name, attribute_id)) = ctx.visit_datasource_scoped() {
            let attribute = &ctx.asts[attribute_id];
            native_types::visit_model_field_native_type_attribute(
                scalar_field_id,
                datasource_name,
                type_name,
                attribute,
                ctx,
            );
        }
    }

    // @unique
    if ctx.visit_optional_single_attr("unique") {
        visit_field_unique(scalar_field_id, model_data, ctx);
        ctx.validate_visited_arguments();
    }

    // @shardKey
    if ctx.visit_optional_single_attr("shardKey") {
        shard_key::field(ast_model, scalar_field_id, field_id, model_data, ctx);
        ctx.validate_visited_arguments();
    }

    ctx.validate_visited_attributes();
}

fn visit_field_unique(scalar_field_id: ScalarFieldId, model_data: &mut ModelAttributes, ctx: &mut Context<'_>) {
    let mapped_name = match ctx
        .visit_optional_arg("map")
        .and_then(|arg| coerce::string(arg, ctx.diagnostics))
    {
        Some("") => {
            ctx.push_attribute_validation_error("The `map` argument cannot be an empty string.");
            None
        }
        Some(name) => Some(ctx.interner.intern(name)),
        None => None,
    };

    let length = ctx
        .visit_optional_arg("length")
        .and_then(|length| coerce::integer(length, ctx.diagnostics))
        .map(|len| len as u32);

    let sort_order = match ctx
        .visit_optional_arg("sort")
        .and_then(|sort| coerce::constant(sort, ctx.diagnostics))
    {
        Some("Desc") => Some(SortOrder::Desc),
        Some("Asc") => Some(SortOrder::Asc),
        Some(other) => {
            ctx.push_attribute_validation_error(&format!(
                "The `sort` argument can only be `Asc` or `Desc` you provided: {other}."
            ));
            None
        }
        None => None,
    };

    let clustered = validate_clustering_setting(ctx);

    let attribute_id = ctx.current_attribute_id();
    model_data.ast_indexes.push((
        attribute_id.1,
        IndexAttribute {
            r#type: IndexType::Unique,
            fields: vec![FieldWithArgs {
                path: IndexFieldPath::new(scalar_field_id),
                sort_order,
                length,
                operator_class: None,
            }],
            source_field: Some(scalar_field_id),
            mapped_name,
            clustered,
            ..Default::default()
        },
    ))
}

fn visit_relation_field_attributes(rfid: RelationFieldId, ctx: &mut Context<'_>) {
    let RelationField { model_id, field_id, .. } = ctx.types[rfid];
    let ast_field = &ctx.asts[model_id][field_id];
    ctx.visit_attributes((model_id.0, (model_id.1, field_id)));

    // @relation
    // Relation attributes are not required at this stage.
    if ctx.visit_optional_single_attr("relation") {
        visit_relation(model_id, rfid, ctx);
        ctx.validate_visited_arguments();
    }

    // @id
    if ctx.visit_optional_single_attr("id") {
        let msg = format!(
            "The field `{}` is a relation field and cannot be marked with `@id`. Only scalar fields can be declared as id.",
            &ast_field.name(),
        );
        ctx.push_attribute_validation_error(&msg);
        ctx.discard_arguments();
    }

    // @ignore
    if ctx.visit_optional_single_attr("ignore") {
        ctx.types[rfid].is_ignored = true;
        ctx.validate_visited_arguments();
    }

    // @default
    if ctx.visit_optional_single_attr("default") {
        ctx.push_attribute_validation_error("Cannot set a default value on a relation field.");
        ctx.discard_arguments();
    }

    // @map
    if ctx.visit_optional_single_attr("map") {
        ctx.push_attribute_validation_error("The attribute `@map` cannot be used on relation fields.");

        if let Err(err) = ctx.visit_default_arg("name") {
            ctx.push_error(err)
        }
        ctx.discard_arguments();
    }

    // @unique
    if ctx.visit_optional_single_attr("unique") {
        let mut suggested_fields = Vec::new();

        for underlying_field in ctx.types[rfid].fields.iter().flatten() {
            let ScalarField { model_id, field_id, .. } = ctx.types[*underlying_field];
            suggested_fields.push(ctx.asts[model_id][field_id].name());
        }

        let suggestion = match suggested_fields.len() {
            0 => String::new(),
            1 => format!(" Did you mean to put it on `{field}`?", field = suggested_fields[0],),
            _ => {
                format!(
                    " Did you mean to provide `@@unique([{fields}])`?",
                    fields = suggested_fields.join(", "),
                )
            }
        };

        ctx.push_attribute_validation_error(
            &format!(
                "The field `{relation_field_name}` is a relation field and cannot be marked with `unique`. Only scalar fields can be made unique.{suggestion}",
                relation_field_name = ast_field.name(),
                suggestion = suggestion
            )
        );
        ctx.discard_arguments();
    }

    ctx.validate_visited_attributes();
}

fn visit_model_ignore(model_id: crate::ModelId, model_data: &mut ModelAttributes, ctx: &mut Context<'_>) {
    let ignored_field_errors: Vec<_> = ctx
        .types
        .range_model_scalar_fields(model_id)
        .filter(|(_, sf)| sf.is_ignored)
        .map(|(_, sf)| {
            DatamodelError::new_attribute_validation_error(
                "Fields on an already ignored Model do not need an `@ignore` annotation.",
                "@ignore",
                ctx.asts[sf.model_id][sf.field_id].span(),
            )
        })
        .collect();

    for error in ignored_field_errors {
        ctx.push_error(error)
    }

    model_data.is_ignored = true;
}

/// Validate @@fulltext on models
fn model_fulltext(data: &mut ModelAttributes, model_id: crate::ModelId, ctx: &mut Context<'_>) {
    let mut index_attribute = IndexAttribute {
        r#type: IndexType::Fulltext,
        ..Default::default()
    };

    common_index_validations(
        &mut index_attribute,
        model_id,
        FieldResolvingSetup::FollowComposites,
        ctx,
    );

    let mapped_name = match ctx
        .visit_optional_arg("map")
        .and_then(|name| coerce::string(name, ctx.diagnostics))
    {
        Some("") => {
            ctx.push_attribute_validation_error("The `map` argument cannot be an empty string.");
            None
        }
        Some(name) => Some(ctx.interner.intern(name)),
        None => None,
    };

    index_attribute.mapped_name = mapped_name;

    data.ast_indexes.push((ctx.current_attribute_id().1, index_attribute));
}

/// Validate @@index on models.
fn model_index(data: &mut ModelAttributes, model_id: crate::ModelId, ctx: &mut Context<'_>) {
    let mut index_attribute = IndexAttribute {
        r#type: IndexType::Normal,
        ..Default::default()
    };

    common_index_validations(
        &mut index_attribute,
        model_id,
        FieldResolvingSetup::FollowComposites,
        ctx,
    );

    let name = get_name_argument(ctx);

    let mapped_name = match ctx
        .visit_optional_arg("map")
        .and_then(|name| coerce::string(name, ctx.diagnostics))
    {
        Some("") => {
            ctx.push_attribute_validation_error("The `map` argument cannot be an empty string.");
            None
        }
        Some(name) => Some(ctx.interner.intern(name)),
        None => None,
    };

    // We do not want to break existing datamodels for client purposes that
    // use the old `@@index([field], name: "onlydbname")` This would
    // strictly speaking be invalid now since `name` in the index definition
    // translates to the client name (which non-unique indexes do not have)
    // and `map` translates to the db constraint name.
    //
    // To prevent existing datamodels from failing to parse after an upgrade
    // we will keep accepting the `name` property on @@index. If it is
    // present we will interpret it as the `map` property and populate it
    // accordingly. If the datamodel gets freshly rendered it will then be
    // rendered correctly as `map`. We will however error if both `map` and
    // `name` are being used.
    index_attribute.mapped_name = match (name, mapped_name) {
        (Some(_), Some(_)) => {
            ctx.push_attribute_validation_error("The `@@index` attribute accepts the `name` argument as an alias for the `map` argument for legacy reasons. It does not accept both though. Please use the `map` argument to specify the database name of the index.");
            None
        }
        // backwards compatibility, accept name arg on normal indexes and use it as map arg.
        (Some(name), None) => Some(name),
        (None, Some(map)) => Some(map),
        (None, None) => None,
    };

    let algo = match ctx
        .visit_optional_arg("type")
        .and_then(|sort| coerce::constant(sort, ctx.diagnostics))
    {
        Some("BTree") => Some(IndexAlgorithm::BTree),
        Some("Hash") => Some(IndexAlgorithm::Hash),
        Some("Gist") => Some(IndexAlgorithm::Gist),
        Some("Gin") => Some(IndexAlgorithm::Gin),
        Some("SpGist") => Some(IndexAlgorithm::SpGist),
        Some("Brin") => Some(IndexAlgorithm::Brin),
        Some(other) => {
            ctx.push_attribute_validation_error(&format!("Unknown index type: {other}."));
            None
        }
        None => None,
    };

    index_attribute.algorithm = algo;
    index_attribute.clustered = validate_clustering_setting(ctx);
    index_attribute.where_clause = parse_where_clause(model_id, ctx);

    data.ast_indexes.push((ctx.current_attribute_id().1, index_attribute));
}

/// Validate @@unique on models.
fn model_unique(data: &mut ModelAttributes, model_id: crate::ModelId, ctx: &mut Context<'_>) {
    let mut index_attribute = IndexAttribute {
        r#type: IndexType::Unique,
        ..Default::default()
    };

    common_index_validations(
        &mut index_attribute,
        model_id,
        FieldResolvingSetup::FollowComposites,
        ctx,
    );

    let current_attribute = ctx.current_attribute();
    let current_attribute_id = ctx.current_attribute_id();
    let ast_model = &ctx.asts[model_id];
    let name = get_name_argument(ctx);

    let mapped_name = {
        // We do not want to break existing datamodels for client purposes that
        // use the old `@@unique([field], name: "ClientANDdbname")` Since we
        // still parse the name argument and pass it to the client they will
        // keep working Migrate will however get a new generated db name for the
        // constraint in that case and try to change the underlying constraint
        // name
        //
        // We are fine with that since this is not automatically breaking but
        // rather prompts a migration upon the first run on migrate. The client
        // is unaffected by this.
        let mapped_name = match ctx
            .visit_optional_arg("map")
            .and_then(|name| coerce::string(name, ctx.diagnostics))
        {
            Some("") => {
                ctx.push_attribute_validation_error("The `map` argument cannot be an empty string.");
                None
            }
            Some(name) => Some(ctx.interner.intern(name)),
            None => None,
        };

        if let Some(name) = name {
            validate_client_name(current_attribute.span, ast_model.name(), name, "@@unique", ctx);
        }

        mapped_name
    };

    index_attribute.name = name;
    index_attribute.mapped_name = mapped_name;
    index_attribute.clustered = validate_clustering_setting(ctx);
    index_attribute.where_clause = parse_where_clause(model_id, ctx);

    data.ast_indexes.push((current_attribute_id.1, index_attribute));
}

/// Parse the `where` argument for partial indexes.
fn parse_where_clause(model_id: crate::ModelId, ctx: &mut Context<'_>) -> Option<WhereClause> {
    let expression = ctx.visit_optional_arg("where")?;

    // Object syntax: { field: value, ... }
    if let Some((members, _span)) = expression.as_object() {
        if members.is_empty() {
            ctx.push_attribute_validation_error("The `where` argument cannot be an empty object.");
            return None;
        }

        let mut conditions = Vec::new();

        for member in members {
            conditions.push(parse_where_object_member(member, model_id, ctx)?);
        }

        return Some(WhereClause::Object(conditions));
    }

    // raw("...") function call
    if let Some(("raw", args)) = coerce::function(expression, ctx.diagnostics) {
        return parse_raw_where_clause(args, ctx);
    }

    ctx.push_attribute_validation_error(
        "The `where` argument must be either a raw() function call or an object literal, e.g. `where: raw(\"status = 'active'\")` or `where: { active: true }`.",
    );

    None
}

/// Parse raw("...") where clause.
fn parse_raw_where_clause(args: &[ast::Argument], ctx: &mut Context<'_>) -> Option<WhereClause> {
    let Some(first_arg) = args.first() else {
        ctx.push_attribute_validation_error(
            "The `where` argument must be a raw() function with a string argument, e.g. `where: raw(\"status = 'active'\")`.",
        );
        return None;
    };

    let Some(predicate) = coerce::string(&first_arg.value, ctx.diagnostics) else {
        ctx.push_attribute_validation_error(
            "The `where` argument must be a raw() function with a string argument, e.g. `where: raw(\"status = 'active'\")`.",
        );
        return None;
    };

    if predicate.is_empty() {
        ctx.push_attribute_validation_error("The `where` argument cannot contain an empty string.");
        return None;
    }

    Some(WhereClause::Raw(predicate.to_string()))
}

fn parse_where_object_member(
    member: &ast::ObjectMember,
    model_id: crate::ModelId,
    ctx: &mut Context<'_>,
) -> Option<WhereFieldCondition> {
    let field_name_str = &member.key;
    let ast_model = &ctx.asts[model_id];

    let field_id = match ctx.find_model_field(model_id, field_name_str) {
        Some(id) => id,
        None => {
            ctx.push_attribute_validation_error(&format!(
                "Field '{}' does not exist in model '{}'.",
                field_name_str,
                ast_model.name()
            ));
            return None;
        }
    };

    let scalar_field_id = match ctx.types.find_model_scalar_field(model_id, field_id) {
        Some(id) => id,
        None => {
            ctx.push_attribute_validation_error(&format!(
                "Field '{}' is a relation field. Only scalar fields can be used in the where clause.",
                field_name_str,
            ));
            return None;
        }
    };

    let scalar_type = ctx.types[scalar_field_id].r#type;

    let condition = parse_where_value(&member.value, field_name_str, scalar_type, false, ctx)?;

    Some(WhereFieldCondition {
        scalar_field_id,
        condition,
    })
}

fn parse_where_value(
    expr: &ast::Expression,
    field_name: &str,
    scalar_type: ScalarFieldType,
    negated: bool,
    ctx: &mut Context<'_>,
) -> Option<WhereCondition> {
    let wrap = |value| {
        if negated {
            WhereCondition::NotEquals(value)
        } else {
            WhereCondition::Equals(value)
        }
    };

    match expr {
        ast::Expression::ConstantValue(val, _) => match val.as_str() {
            "true" | "false" => {
                check_type(field_name, scalar_type, &[ScalarType::Boolean], "Boolean", ctx)?;
                Some(wrap(WhereValue::Boolean(val == "true")))
            }
            "null" => Some(if negated {
                WhereCondition::IsNotNull
            } else {
                WhereCondition::IsNull
            }),
            other => {
                ctx.push_attribute_validation_error(&format!(
                    "Invalid value '{other}' in where clause. Expected true, false, null, a string, a number, or an object like {{ not: null }}."
                ));
                None
            }
        },
        ast::Expression::StringValue(val, _) => {
            check_type(
                field_name,
                scalar_type,
                &[ScalarType::String, ScalarType::DateTime],
                "a String",
                ctx,
            )?;
            Some(wrap(WhereValue::String(val.clone())))
        }
        ast::Expression::NumericValue(val, _) => {
            check_type(
                field_name,
                scalar_type,
                &[
                    ScalarType::Int,
                    ScalarType::BigInt,
                    ScalarType::Float,
                    ScalarType::Decimal,
                ],
                "a Number",
                ctx,
            )?;
            Some(wrap(WhereValue::Number(val.clone())))
        }
        ast::Expression::Object(inner_members, _) if !negated => {
            if inner_members.len() != 1 {
                ctx.push_attribute_validation_error(
                    "Nested object in where clause must have exactly one key. Use `{ not: null }` or `{ not: \"value\" }`.",
                );
                return None;
            }

            let inner = &inner_members[0];
            if inner.key != "not" {
                ctx.push_attribute_validation_error(&format!(
                    "Unknown key '{}' in nested where clause object. Only 'not' is supported.",
                    inner.key
                ));
                return None;
            }

            parse_where_value(&inner.value, field_name, scalar_type, true, ctx)
        }
        _ => {
            ctx.push_attribute_validation_error(
                "Invalid value in where clause. Expected true, false, null, a string, a number, or an object like { not: null }.",
            );
            None
        }
    }
}

fn check_type(
    field_name: &str,
    scalar_type: ScalarFieldType,
    accepted: &[ScalarType],
    value_type: &str,
    ctx: &mut Context<'_>,
) -> Option<()> {
    match scalar_type {
        ScalarFieldType::BuiltInScalar(t) if accepted.contains(&t) => Some(()),
        ScalarFieldType::BuiltInScalar(t) => {
            ctx.push_attribute_validation_error(&format!(
                "Type mismatch: field '{}' is of type {}, but the value is {}.",
                field_name,
                t.as_str(),
                value_type,
            ));
            None
        }
        ScalarFieldType::Enum(_) if accepted.contains(&ScalarType::String) => Some(()),
        ScalarFieldType::Enum(_) => {
            ctx.push_attribute_validation_error(&format!(
                "Type mismatch: field '{}' is an Enum and only accepts String values in the where clause.",
                field_name,
            ));
            None
        }
        _ => {
            let type_name = match scalar_type {
                ScalarFieldType::CompositeType(_) => "a composite type",
                ScalarFieldType::Unsupported(_) => "an unsupported type",
                _ => "a non-scalar type",
            };
            ctx.push_attribute_validation_error(&format!(
                "Field '{}' is {} and cannot be used in the object syntax of a where clause. Use raw() instead.",
                field_name, type_name,
            ));
            None
        }
    }
}

fn common_index_validations(
    index_data: &mut IndexAttribute,
    model_id: crate::ModelId,
    resolving: FieldResolvingSetup,
    ctx: &mut Context<'_>,
) {
    let current_attribute = ctx.current_attribute();
    let fields = match ctx.visit_default_arg("fields") {
        Ok(fields) => fields,
        Err(err) => {
            return ctx.push_error(err);
        }
    };

    match resolve_field_array_with_args(fields, current_attribute.span, model_id, resolving, ctx) {
        Ok(fields) => {
            index_data.fields = fields;
        }
        Err(FieldResolutionError::AlreadyDealtWith) => (),
        Err(FieldResolutionError::ProblematicFields {
            unknown_fields: unresolvable_fields,
            relation_fields,
        }) => {
            if !unresolvable_fields.is_empty() {
                let fields = unresolvable_fields
                    .iter()
                    .map(|((file_id, top_id), field_name)| match top_id {
                        ast::TopId::CompositeType(ctid) => {
                            let composite_type = &ctx.asts[(*file_id, *ctid)].name();

                            Cow::from(format!("{field_name} in type {composite_type}"))
                        }
                        ast::TopId::Model(_) => Cow::from(*field_name),
                        _ => unreachable!(),
                    })
                    .collect::<Vec<_>>();

                ctx.push_error({
                    let message: &str = &format!(
                        "The {}index definition refers to the unknown fields: {}.",
                        if index_data.is_unique() { "unique " } else { "" },
                        fields.join(", "),
                    );
                    let model_name = ctx.asts[model_id].name();
                    DatamodelError::new_model_validation_error(message, "model", model_name, current_attribute.span)
                });
            }

            if !relation_fields.is_empty() {
                let mut suggested_fields = Vec::new();

                for (_, field_id) in &relation_fields {
                    let Some(rf) = ctx
                        .types
                        .range_model_relation_fields(model_id)
                        .find(|(_, rf)| rf.field_id == *field_id)
                    else {
                        continue;
                    };

                    let fields = rf.1.fields.iter().flatten();

                    for underlying_field in fields {
                        let ScalarField { model_id, field_id, .. } = ctx.types[*underlying_field];
                        suggested_fields.push(ctx.asts[model_id][field_id].name());
                    }
                }

                let suggestion = if !suggested_fields.is_empty() {
                    format!(
                        " Did you mean `@@{attribute_name}([{fields}])`?",
                        attribute_name = if index_data.is_unique() { "unique" } else { "index" },
                        fields = suggested_fields.join(", ")
                    )
                } else {
                    String::new()
                };

                ctx.push_error(DatamodelError::new_model_validation_error(
                    &format!(
                        "The {prefix}index definition refers to the relation fields {the_fields}. Index definitions must reference only scalar fields.{suggestion}",
                        prefix = if index_data.is_unique() { "unique " } else { "" },
                        the_fields = relation_fields.iter().map(|(f, _)| f.name()).collect::<Vec<_>>().join(", "),
                        suggestion = suggestion
                    ),
                    "model",
                    ctx.asts[model_id].name(),
                    current_attribute.span,
                ));
            }
        }
    }
}

/// @relation validation for relation fields.
fn visit_relation(model_id: crate::ModelId, relation_field_id: RelationFieldId, ctx: &mut Context<'_>) {
    let attr = ctx.current_attribute();
    ctx.types[relation_field_id].relation_attribute = Some(ctx.current_attribute_id().1);

    if let Some(fields) = ctx.visit_optional_arg("fields") {
        let fields = match resolve_field_array_without_args(fields, attr.span, model_id, ctx) {
            Ok(fields) => fields,
            Err(FieldResolutionError::AlreadyDealtWith) => Vec::new(),
            Err(FieldResolutionError::ProblematicFields {
                unknown_fields: unresolvable_fields,
                relation_fields,
            }) => {
                if !unresolvable_fields.is_empty() {
                    let unresolvable_fields = unresolvable_fields
                        .into_iter()
                        .map(|(_, field)| field)
                        .collect::<Vec<_>>()
                        .join(", ");

                    let msg = format!(
                        "The argument fields must refer only to existing fields. The following fields do not exist in this model: {unresolvable_fields}"
                    );

                    ctx.push_error(DatamodelError::new_validation_error(&msg, fields.span()))
                }

                if !relation_fields.is_empty() {
                    let relation_fields = relation_fields
                        .into_iter()
                        .map(|(f, _)| f.name())
                        .collect::<Vec<_>>()
                        .join(", ");

                    let msg = format!(
                        "The argument fields must refer only to scalar fields. But it is referencing the following relation fields: {relation_fields}"
                    );

                    ctx.push_error(DatamodelError::new_validation_error(&msg, fields.span()));
                }

                Vec::new()
            }
        };

        ctx.types[relation_field_id].fields = Some(fields);
    }

    if let Some(references) = ctx.visit_optional_arg("references") {
        let references = match resolve_field_array_without_args(
            references,
            attr.span,
            ctx.types[relation_field_id].referenced_model,
            ctx,
        ) {
            Ok(references) => references,
            Err(FieldResolutionError::AlreadyDealtWith) => Vec::new(),
            Err(FieldResolutionError::ProblematicFields {
                relation_fields,
                unknown_fields,
            }) => {
                if !unknown_fields.is_empty() {
                    let model_name = ctx.asts[ctx.types[relation_field_id].referenced_model].name();

                    let field_names = unknown_fields
                        .into_iter()
                        .map(|(_, field_name)| field_name)
                        .collect::<Vec<_>>()
                        .join(", ");

                    let msg = format!(
                        "The argument `references` must refer only to existing fields in the related model `{model_name}`. The following fields do not exist in the related model: {field_names}",
                    );

                    ctx.push_error(DatamodelError::new_validation_error(&msg, attr.span));
                }

                if !relation_fields.is_empty() {
                    let msg = format!(
                        "The argument `references` must refer only to scalar fields in the related model `{}`. But it is referencing the following relation fields: {}",
                        ctx.asts[ctx.types[relation_field_id].referenced_model].name(),
                        relation_fields
                            .iter()
                            .map(|(f, _)| f.name())
                            .collect::<Vec<_>>()
                            .join(", "),
                    );
                    ctx.push_error(DatamodelError::new_validation_error(&msg, attr.span));
                }

                Vec::new()
            }
        };

        ctx.types[relation_field_id].references = Some(references);
    }

    // Validate the `name` argument if present.
    match ctx
        .visit_default_arg("name")
        .ok()
        .and_then(|arg| coerce::string(arg, ctx.diagnostics))
    {
        Some("") => ctx.push_attribute_validation_error("A relation cannot have an empty name."),
        Some(name) => {
            let interned_name = ctx.interner.intern(name);
            ctx.types[relation_field_id].name = Some(interned_name);
        }
        None => (),
    }

    // Validate referential actions.
    if let Some(on_delete) = ctx.visit_optional_arg("onDelete")
        && let Some(action) = crate::ReferentialAction::try_from_expression(on_delete, ctx.diagnostics)
    {
        ctx.types[relation_field_id].on_delete = Some((action, on_delete.span()));
    }

    if let Some(on_update) = ctx.visit_optional_arg("onUpdate")
        && let Some(action) = crate::ReferentialAction::try_from_expression(on_update, ctx.diagnostics)
    {
        ctx.types[relation_field_id].on_update = Some((action, on_update.span()));
    }

    let fk_name = {
        match ctx
            .visit_optional_arg("map")
            .and_then(|name| coerce::string(name, ctx.diagnostics))
        {
            Some("") => {
                ctx.push_attribute_validation_error("The `map` argument cannot be an empty string.");
                None
            }
            Some(name) => Some(ctx.interner.intern(name)),
            None => None,
        }
    };

    ctx.types[relation_field_id].mapped_name = fk_name;
}

#[derive(Debug)]
enum FieldResolutionError<'ast> {
    AlreadyDealtWith,
    ProblematicFields {
        /// Fields that do not exist on the model.
        unknown_fields: Vec<(crate::TopId, &'ast str)>,
        /// Fields that exist on the model but are relation fields.
        relation_fields: Vec<(&'ast ast::Field, ast::FieldId)>,
    },
}

/// Takes an attribute argument, validates it as an array of constants, then
/// resolves  the constant as field names on the model. The error variant
/// contains the fields that are not in the model.
fn resolve_field_array_without_args<'db>(
    values: &'db ast::Expression,
    attribute_span: ast::Span,
    model_id: crate::ModelId,
    ctx: &mut Context<'db>,
) -> Result<Vec<ScalarFieldId>, FieldResolutionError<'db>> {
    let file_id = model_id.0;
    let constant_array = match coerce_array(values, &coerce::constant, ctx.diagnostics) {
        Some(values) => values,
        None => {
            return Err(FieldResolutionError::AlreadyDealtWith);
        }
    };

    let mut field_ids: Vec<ScalarFieldId> = Vec::with_capacity(constant_array.len());
    let mut unknown_fields = Vec::new();
    let mut relation_fields = Vec::new();
    let ast_model = &ctx.asts[model_id];

    for field_name in constant_array {
        if field_name.contains('.') {
            unknown_fields.push(((file_id, ast::TopId::Model(model_id.1)), field_name));
            continue;
        }

        // Does the field exist?
        let field_id = if let Some(field_id) = ctx.find_model_field(model_id, field_name) {
            field_id
        } else {
            unknown_fields.push(((file_id, ast::TopId::Model(model_id.1)), field_name));
            continue;
        };

        // Is the field a scalar field?
        let sfid = if let Some(sfid) = ctx.types.find_model_scalar_field(model_id, field_id) {
            sfid
        } else {
            relation_fields.push((&ctx.asts[model_id][field_id], field_id));
            continue;
        };

        // Is the field used twice?
        if field_ids.contains(&sfid) {
            ctx.push_error(DatamodelError::new_model_validation_error(
                &format!(
                    "The unique index definition refers to the field {} multiple times.",
                    ast_model[field_id].name()
                ),
                "model",
                ast_model.name(),
                attribute_span,
            ));
            return Err(FieldResolutionError::AlreadyDealtWith);
        }

        field_ids.push(sfid);
    }

    if !unknown_fields.is_empty() || !relation_fields.is_empty() {
        Err(FieldResolutionError::ProblematicFields {
            unknown_fields,
            relation_fields,
        })
    } else {
        Ok(field_ids)
    }
}

#[derive(Debug, Clone, Copy)]
enum FieldResolvingSetup {
    OnlyTopLevel,
    FollowComposites,
}

impl FieldResolvingSetup {
    fn follow_composites(self) -> bool {
        matches!(self, Self::FollowComposites)
    }
}

/// Takes an attribute argument, validates it as an array of fields with potentially args,
/// then resolves  the constant literal as field names on the model. The error variant
/// contains the fields that are not in the model.
fn resolve_field_array_with_args<'db>(
    values: &'db ast::Expression,
    attribute_span: ast::Span,
    model_id: crate::ModelId,
    resolving: FieldResolvingSetup,
    ctx: &mut Context<'db>,
) -> Result<Vec<FieldWithArgs>, FieldResolutionError<'db>> {
    let file_id = model_id.0;
    let constant_array = match crate::types::index_fields::coerce_field_array_with_args(values, ctx.diagnostics) {
        Some(values) => values,
        None => return Err(FieldResolutionError::AlreadyDealtWith),
    };

    let mut field_ids = Vec::with_capacity(constant_array.len());
    let mut unknown_fields = Vec::new();
    let mut relation_fields = Vec::new();

    let ast_model = &ctx.asts[model_id];

    'fields: for attrs in &constant_array {
        let path = if attrs.field_name.contains('.') {
            if !resolving.follow_composites() {
                unknown_fields.push(((file_id, ast::TopId::Model(model_id.1)), attrs.field_name));
                continue 'fields;
            }

            let field_count = attrs.field_name.split('.').count();
            let mut path_in_schema = attrs.field_name.split('.').enumerate();

            let (mut path, mut next_type) = match path_in_schema.next() {
                Some((_, field_shard)) => {
                    let field_id = match ctx.find_model_field(model_id, field_shard) {
                        Some(field_id) => field_id,
                        None => {
                            unknown_fields.push(((file_id, ast::TopId::Model(model_id.1)), field_shard));
                            continue 'fields;
                        }
                    };

                    let sfid = if let Some(sfid) = ctx.types.find_model_scalar_field(model_id, field_id) {
                        sfid
                    } else {
                        relation_fields.push((&ctx.asts[model_id][field_id], field_id));
                        continue 'fields;
                    };

                    match &ctx.types[sfid].r#type {
                        ScalarFieldType::CompositeType(ctid) => (IndexFieldPath::new(sfid), ctid),
                        _ => {
                            unknown_fields.push(((file_id, ast::TopId::Model(model_id.1)), attrs.field_name));
                            continue 'fields;
                        }
                    }
                }
                None => {
                    // TODO: See if we need to actually error here, or if this
                    // case is handled earlier.
                    continue 'fields;
                }
            };

            for (i, field_shard) in path_in_schema {
                let field_id = match ctx.find_composite_type_field(*next_type, field_shard) {
                    Some(field_id) => field_id,
                    None => {
                        unknown_fields.push(((next_type.0, ast::TopId::CompositeType(next_type.1)), field_shard));
                        continue 'fields;
                    }
                };

                path.push_field(*next_type, field_id);

                match &ctx.types.composite_type_fields[&(*next_type, field_id)].r#type {
                    ScalarFieldType::CompositeType(ctid) => {
                        next_type = ctid;
                    }
                    _ if i < field_count - 1 => {
                        unknown_fields.push(((model_id.0, ast::TopId::Model(model_id.1)), attrs.field_name));
                        continue 'fields;
                    }
                    _ => (),
                }
            }

            path
        } else if let Some(field_id) = ctx.find_model_field(model_id, attrs.field_name) {
            // Is the field a scalar field?
            match ctx.types.find_model_scalar_field(model_id, field_id) {
                Some(sfid) => IndexFieldPath::new(sfid),
                None => {
                    relation_fields.push((&ctx.asts[model_id][field_id], field_id));
                    continue;
                }
            }
        } else {
            unknown_fields.push(((model_id.0, ast::TopId::Model(model_id.1)), attrs.field_name));
            continue;
        };

        // Is the field used twice?
        if field_ids.contains(&path) {
            let path_str = match path.field_in_index() {
                either::Either::Left(_) => Cow::from(attrs.field_name),
                either::Either::Right((ctid, field_id)) => {
                    let field_name = &ctx.asts[ctid][field_id].name();
                    let composite_type = &ctx.asts[ctid].name();

                    Cow::from(format!("{field_name} in type {composite_type}"))
                }
            };

            ctx.push_error(DatamodelError::new_model_validation_error(
                &format!("The unique index definition refers to the field {path_str} multiple times.",),
                "model",
                ast_model.name(),
                attribute_span,
            ));

            return Err(FieldResolutionError::AlreadyDealtWith);
        }

        field_ids.push(path);
    }

    if !unknown_fields.is_empty() || !relation_fields.is_empty() {
        Err(FieldResolutionError::ProblematicFields {
            unknown_fields,
            relation_fields,
        })
    } else {
        let fields_with_args = constant_array
            .into_iter()
            .zip(field_ids)
            .map(|(attrs, field_location)| FieldWithArgs {
                path: field_location,
                sort_order: attrs.sort_order,
                length: attrs.length,
                operator_class: attrs.operator_class.map(|c| convert_op_class(c, ctx)),
            })
            .collect();

        Ok(fields_with_args)
    }
}

fn convert_op_class(raw: crate::types::index_fields::OperatorClass<'_>, ctx: &mut Context<'_>) -> OperatorClassStore {
    match raw {
        crate::types::index_fields::OperatorClass::Constant(class) => OperatorClassStore::from(class),
        crate::types::index_fields::OperatorClass::Raw(s) => OperatorClassStore::raw(ctx.interner.intern(s)),
    }
}

fn get_name_argument(ctx: &mut Context<'_>) -> Option<StringId> {
    match ctx
        .visit_optional_arg("name")
        .and_then(|name| coerce::string(name, ctx.diagnostics))
    {
        Some("") => {
            ctx.push_attribute_validation_error("The `name` argument cannot be an empty string.");
            None
        }
        Some(name) => Some(ctx.interner.intern(name)),
        None => None,
    }
}

fn validate_client_name(span: Span, object_name: &str, name: StringId, attribute: &'static str, ctx: &mut Context<'_>) {
    // only Alphanumeric characters and underscore are allowed due to this making its way into the client API
    // todo what about starting with a number or underscore?
    {
        let name = &ctx[name];

        let is_valid = name
            .chars()
            .all(|c| c == '_' || c.is_ascii_digit() || c.is_ascii_alphabetic());

        if is_valid {
            return;
        }
    }

    ctx.push_error(DatamodelError::new_model_validation_error(
        &format!(
            "The `name` property within the `{attribute}` attribute only allows for the following characters: `_a-zA-Z0-9`."
        ),
        "model",
        object_name,
        span,
    ))
}

fn validate_clustering_setting(ctx: &mut Context<'_>) -> Option<bool> {
    ctx.visit_optional_arg("clustered")
        .and_then(|sort| coerce::boolean(sort, ctx.diagnostics))
}

fn format_fields_in_error_with_leading_word<'a>(
    fields: impl IntoIterator<IntoIter: ExactSizeIterator<Item = impl Display + 'a> + 'a>,
) -> impl Display + 'a {
    struct Format<I>(Cell<Option<I>>);

    impl<F, I> Display for Format<I>
    where
        F: Display,
        I: IntoIterator<IntoIter: ExactSizeIterator<Item = F>>,
    {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let Some(iter) = self.0.take().map(<_>::into_iter) else {
                panic!("`format_fields_in_error_with_leading_word` result can only be formatted once")
            };
            write!(f, "field")?;
            if iter.len() > 1 {
                write!(f, "s")?;
            }
            write!(f, " {}", iter.map(Field).format(", "))
        }
    }

    struct Field<D>(D);

    impl<D: Display> Display for Field<D> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "`{}`", self.0)
        }
    }

    Format(Cell::new(Some(fields.into_iter())))
}

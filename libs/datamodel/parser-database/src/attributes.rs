mod default;
mod id;
mod map;
mod native_types;

use std::borrow::Cow;

use crate::{
    ast::{self, WithName, WithSpan},
    context::Context,
    types::{
        EnumAttributes, FieldWithArgs, IndexAlgorithm, IndexAttribute, IndexFieldPath, IndexType, ModelAttributes,
        RelationField, ScalarField, ScalarFieldType, SortOrder,
    },
    DatamodelError, StringId, ValueValidator,
};
use diagnostics::Span;

pub(super) fn resolve_attributes(ctx: &mut Context<'_>) {
    for top in ctx.ast.iter_tops() {
        match top {
            (ast::TopId::Model(model_id), ast::Top::Model(model)) => resolve_model_attributes(model_id, model, ctx),
            (ast::TopId::Enum(enum_id), ast::Top::Enum(ast_enum)) => resolve_enum_attributes(enum_id, ast_enum, ctx),
            (ast::TopId::CompositeType(ctid), ast::Top::CompositeType(ct)) => {
                resolve_composite_type_attributes(ctid, ct, ctx)
            }
            _ => (),
        }
    }
}

fn resolve_composite_type_attributes<'db>(
    ctid: ast::CompositeTypeId,
    ct: &'db ast::CompositeType,
    ctx: &mut Context<'db>,
) {
    for (field_id, field) in ct.iter_fields() {
        let mut ctfield = ctx.types.composite_type_fields[&(ctid, field_id)].clone();

        ctx.visit_attributes((ctid, field_id).into());

        if let ScalarFieldType::BuiltInScalar(_scalar_type) = ctfield.r#type {
            // native type attributes
            if let Some((datasource_name, type_name, args)) = ctx.visit_datasource_scoped() {
                native_types::visit_composite_type_field_native_type_attribute(
                    datasource_name,
                    type_name,
                    &ctx.ast[args],
                    &mut ctfield,
                )
            }
        }

        // @map
        if ctx.visit_optional_single_attr("map") {
            map::composite_type_field(ct, field, ctid, field_id, &mut ctfield, ctx);
            ctx.validate_visited_arguments();
        }

        // @default
        if ctx.visit_optional_single_attr("default") {
            default::visit_composite_field_default(&mut ctfield, ctid, field_id, ctx);
            ctx.validate_visited_arguments();
        }

        ctx.types.composite_type_fields.insert((ctid, field_id), ctfield);
        ctx.validate_visited_attributes();
    }
}

fn resolve_enum_attributes<'db>(enum_id: ast::EnumId, ast_enum: &'db ast::Enum, ctx: &mut Context<'db>) {
    let mut enum_attributes = EnumAttributes::default();

    for value_idx in 0..ast_enum.values.len() {
        ctx.visit_attributes((enum_id, value_idx as u32).into());
        // @map
        if ctx.visit_optional_single_attr("map") {
            if let Some(mapped_name) = map::visit_map_attribute(ctx) {
                enum_attributes.mapped_values.insert(value_idx as u32, mapped_name);
                ctx.mapped_enum_value_names
                    .insert((enum_id, mapped_name), value_idx as u32);
            }
            ctx.validate_visited_arguments();
        }
        ctx.validate_visited_attributes();
    }

    // Now validate the enum attributes.

    ctx.visit_attributes(enum_id.into());

    // @@map
    if ctx.visit_optional_single_attr("map") {
        if let Some(mapped_name) = map::visit_map_attribute(ctx) {
            enum_attributes.mapped_name = Some(mapped_name);
            ctx.mapped_enum_names.insert(mapped_name, enum_id);
        }
        ctx.validate_visited_arguments();
    }

    ctx.types.enum_attributes.insert(enum_id, enum_attributes);
    ctx.validate_visited_attributes();
}

fn resolve_model_attributes<'db>(model_id: ast::ModelId, ast_model: &'db ast::Model, ctx: &mut Context<'db>) {
    let mut model_attributes = ModelAttributes::default();

    // First resolve all the attributes defined on fields **in isolation**.
    for (field_id, ast_field) in ast_model.iter_fields() {
        if let Some(mut scalar_field) = ctx.types.take_scalar_field(model_id, field_id) {
            visit_scalar_field_attributes(
                model_id,
                field_id,
                ast_model,
                ast_field,
                &mut model_attributes,
                &mut scalar_field,
                ctx,
            );

            ctx.types.scalar_fields.insert((model_id, field_id), scalar_field);
        } else if let Some(mut rf) = ctx.types.take_relation_field(model_id, field_id) {
            visit_relation_field_attributes(model_id, field_id, ast_field, &mut rf, ctx);
            ctx.types.relation_fields.insert((model_id, field_id), rf);
        } else {
            unreachable!(
                "{}.{} is neither a scalar field nor a relation field",
                ast_model.name(),
                ast_field.name()
            )
        }
    }

    // Resolve all the attributes defined on the model itself **in isolation**.
    ctx.visit_attributes(model_id.into());

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
        map::model(&mut model_attributes, model_id, ctx);
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

    // Model-global validations
    id::validate_id_field_arities(model_id, &model_attributes, ctx);

    ctx.types.model_attributes.insert(model_id, model_attributes);
    ctx.validate_visited_attributes();
}

fn visit_scalar_field_attributes<'db>(
    model_id: ast::ModelId,
    field_id: ast::FieldId,
    ast_model: &'db ast::Model,
    ast_field: &'db ast::Field,
    model_attributes: &mut ModelAttributes,
    scalar_field_data: &mut ScalarField,
    ctx: &mut Context<'db>,
) {
    ctx.visit_scalar_field_attributes(model_id, field_id, scalar_field_data.r#type);

    // @map
    if ctx.visit_optional_single_attr("map") {
        map::scalar_field(ast_model, ast_field, model_id, field_id, scalar_field_data, ctx);
        ctx.validate_visited_arguments();
    }

    // @ignore
    if ctx.visit_optional_single_attr("ignore") {
        if matches!(scalar_field_data.r#type, ScalarFieldType::Unsupported(_)) {
            ctx.push_attribute_validation_error("Fields of type `Unsupported` cannot take an `@ignore` attribute. They are already treated as ignored by the client due to their type.");
        } else {
            scalar_field_data.is_ignored = true;
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
        id::field(ast_model, field_id, model_attributes, ctx);
        ctx.validate_visited_arguments();
    }

    // @updatedAt
    if ctx.visit_optional_single_attr("updatedAt") {
        if !matches!(
            scalar_field_data.r#type,
            ScalarFieldType::BuiltInScalar(crate::ScalarType::DateTime)
        ) {
            ctx.push_attribute_validation_error("Fields that are marked with @updatedAt must be of type DateTime.");
        }

        if ast_field.arity.is_list() {
            ctx.push_attribute_validation_error("Fields that are marked with @updatedAt cannot be lists.");
        }

        scalar_field_data.is_updated_at = true;
        ctx.validate_visited_arguments();
    }

    // @default
    if ctx.visit_optional_single_attr("default") {
        default::visit_model_field_default(scalar_field_data, model_id, field_id, ctx);
        ctx.validate_visited_arguments();
    }

    if let ScalarFieldType::BuiltInScalar(_scalar_type) = scalar_field_data.r#type {
        // native type attributes
        if let Some((datasource_name, type_name, attribute_id)) = ctx.visit_datasource_scoped() {
            let attribute = &ctx.ast[attribute_id];
            native_types::visit_model_field_native_type_attribute(
                datasource_name,
                type_name,
                attribute,
                scalar_field_data,
            );
        }
    }

    // @unique
    if ctx.visit_optional_single_attr("unique") {
        visit_field_unique(field_id, model_attributes, ctx);
        ctx.validate_visited_arguments();
    }

    ctx.validate_visited_attributes();
}

fn visit_field_unique(field_id: ast::FieldId, model_attributes: &mut ModelAttributes, ctx: &mut Context<'_>) {
    let mapped_name = match ctx.visit_optional_arg("map").map(|name| name.as_str()) {
        Some(Ok("")) => {
            ctx.push_attribute_validation_error("The `map` argument cannot be an empty string.");
            None
        }
        Some(Ok(name)) => Some(ctx.interner.intern(name)),
        Some(Err(err)) => {
            ctx.push_error(err);
            None
        }
        None => None,
    };

    let length = match ctx.visit_optional_arg("length").map(|length| length.as_int()) {
        Some(Ok(length)) => Some(length as u32),
        Some(Err(err)) => {
            ctx.push_error(err);
            None
        }
        None => None,
    };

    let sort_order = match ctx.visit_optional_arg("sort").map(|sort| sort.as_constant_literal()) {
        Some(Ok("Desc")) => Some(SortOrder::Desc),
        Some(Ok("Asc")) => Some(SortOrder::Asc),
        Some(Ok(other)) => {
            ctx.push_attribute_validation_error(&format!(
                "The `sort` argument can only be `Asc` or `Desc` you provided: {}.",
                other
            ));
            None
        }
        Some(Err(err)) => {
            ctx.push_error(err);
            None
        }
        None => None,
    };

    let clustered = validate_clustering_setting(ctx);

    model_attributes.ast_indexes.push((
        ctx.current_attribute_id(),
        IndexAttribute {
            r#type: IndexType::Unique,
            fields: vec![FieldWithArgs {
                path: IndexFieldPath::new(field_id),
                sort_order,
                length,
            }],
            source_field: Some(field_id),
            mapped_name,
            clustered,
            ..Default::default()
        },
    ))
}

fn visit_relation_field_attributes<'db>(
    model_id: ast::ModelId,
    field_id: ast::FieldId,
    ast_field: &'db ast::Field,
    relation_field: &mut RelationField,
    ctx: &mut Context<'db>,
) {
    ctx.visit_attributes((model_id, field_id).into());

    // @relation
    // Relation attributes are not required at this stage.
    if ctx.visit_optional_single_attr("relation") {
        visit_relation(model_id, relation_field, ctx);
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
        relation_field.is_ignored = true;
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

        for underlying_field in relation_field.fields.iter().flatten() {
            suggested_fields.push(ctx.ast[model_id][*underlying_field].name());
        }

        let suggestion = match suggested_fields.len() {
            0 => String::new(),
            1 => format!(" Did you mean to put it on `{field}`?", field = suggested_fields[0],),
            _ => {
                format!(
                    " Did you mean to provide `@@unique([{}])`?",
                    field = suggested_fields.join(", ")
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

fn visit_model_ignore(model_id: ast::ModelId, model_data: &mut ModelAttributes, ctx: &mut Context<'_>) {
    let ignored_field_errors: Vec<_> = ctx
        .types
        .range_model_scalar_fields(model_id)
        .filter(|(_, sf)| sf.is_ignored)
        .map(|(field_id, _)| {
            DatamodelError::new_attribute_validation_error(
                "Fields on an already ignored Model do not need an `@ignore` annotation.",
                "ignore",
                *ctx.ast[model_id][field_id].span(),
            )
        })
        .collect();

    for error in ignored_field_errors {
        ctx.push_error(error)
    }

    model_data.is_ignored = true;
}

/// Validate @@fulltext on models
fn model_fulltext(data: &mut ModelAttributes, model_id: ast::ModelId, ctx: &mut Context<'_>) {
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

    let mapped_name = match ctx.visit_optional_arg("map").map(|name| name.as_str()) {
        Some(Ok("")) => {
            ctx.push_attribute_validation_error("The `map` argument cannot be an empty string.");
            None
        }
        Some(Ok(name)) => Some(ctx.interner.intern(name)),
        Some(Err(err)) => {
            ctx.push_error(err);
            None
        }
        None => None,
    };

    index_attribute.mapped_name = mapped_name;

    data.ast_indexes.push((ctx.current_attribute_id(), index_attribute));
}

/// Validate @@index on models.
fn model_index(data: &mut ModelAttributes, model_id: ast::ModelId, ctx: &mut Context<'_>) {
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

    let mapped_name = match ctx.visit_optional_arg("map").map(|name| name.as_str()) {
        Some(Ok("")) => {
            ctx.push_attribute_validation_error("The `map` argument cannot be an empty string.");
            None
        }
        Some(Ok(name)) => Some(ctx.interner.intern(name)),
        Some(Err(err)) => {
            ctx.push_error(err);
            None
        }
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

    let algo = match ctx.visit_optional_arg("type").map(|sort| sort.as_constant_literal()) {
        Some(Ok("BTree")) => Some(IndexAlgorithm::BTree),
        Some(Ok("Hash")) => Some(IndexAlgorithm::Hash),
        Some(Ok(other)) => {
            ctx.push_attribute_validation_error(&format!("Unknown index type: {}.", other));
            None
        }
        Some(Err(err)) => {
            ctx.push_error(err);
            None
        }
        None => None,
    };

    index_attribute.algorithm = algo;
    index_attribute.clustered = validate_clustering_setting(ctx);

    data.ast_indexes.push((ctx.current_attribute_id(), index_attribute));
}

/// Validate @@unique on models.
fn model_unique(data: &mut ModelAttributes, model_id: ast::ModelId, ctx: &mut Context<'_>) {
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
    let ast_model = &ctx.ast[model_id];
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
        let mapped_name = match ctx.visit_optional_arg("map").map(|name| name.as_str()) {
            Some(Ok("")) => {
                ctx.push_attribute_validation_error("The `map` argument cannot be an empty string.");
                None
            }
            Some(Ok(name)) => Some(ctx.interner.intern(name)),
            Some(Err(err)) => {
                ctx.push_error(err);
                None
            }
            None => None,
        };

        if let Some(name) = name {
            validate_client_name(current_attribute.span, &ast_model.name.name, name, "@@unique", ctx);
        }

        mapped_name
    };

    index_attribute.name = name;
    index_attribute.mapped_name = mapped_name;
    index_attribute.clustered = validate_clustering_setting(ctx);

    data.ast_indexes.push((current_attribute_id, index_attribute));
}

fn common_index_validations(
    index_data: &mut IndexAttribute,
    model_id: ast::ModelId,
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

    match resolve_field_array_with_args(&fields, current_attribute.span, model_id, resolving, ctx) {
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
                    .map(|(top_id, field_name)| match top_id {
                        ast::TopId::CompositeType(ctid) => {
                            let composite_type = &ctx.ast[*ctid].name.name;

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
                    let model_name = ctx.ast[model_id].name();
                    DatamodelError::new_model_validation_error(message, model_name, current_attribute.span)
                });
            }

            if !relation_fields.is_empty() {
                let mut suggested_fields = Vec::new();

                for (_, field_id) in &relation_fields {
                    let relation_field = &ctx.types.relation_fields[&(model_id, *field_id)];
                    for underlying_field in relation_field.fields.iter().flatten() {
                        suggested_fields.push(ctx.ast[model_id][*underlying_field].name());
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
                    ctx.ast[model_id].name(),
                    current_attribute.span,
                ));
            }
        }
    }
}

/// @relation validation for relation fields.
fn visit_relation(model_id: ast::ModelId, relation_field: &mut RelationField, ctx: &mut Context<'_>) {
    let attr = ctx.current_attribute();
    relation_field.relation_attribute = Some(ctx.current_attribute_id());

    if let Some(fields) = ctx.visit_optional_arg("fields") {
        let fields = match resolve_field_array_without_args(&fields, attr.span, model_id, ctx) {
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

                    let msg = format!("The argument fields must refer only to existing fields. The following fields do not exist in this model: {unresolvable_fields}");

                    ctx.push_error(DatamodelError::new_validation_error(msg, fields.span()))
                }

                if !relation_fields.is_empty() {
                    let relation_fields = relation_fields
                        .into_iter()
                        .map(|(f, _)| f.name())
                        .collect::<Vec<_>>()
                        .join(", ");

                    let msg = format!("The argument fields must refer only to scalar fields. But it is referencing the following relation fields: {relation_fields}");

                    ctx.push_error(DatamodelError::new_validation_error(msg, fields.span()));
                }

                Vec::new()
            }
        };

        relation_field.fields = Some(fields);
    }

    if let Some(references) = ctx.visit_optional_arg("references") {
        let references = match resolve_field_array_without_args(
            &references,
            attr.span,
            relation_field.referenced_model,
            ctx,
        ) {
            Ok(references) => references,
            Err(FieldResolutionError::AlreadyDealtWith) => Vec::new(),
            Err(FieldResolutionError::ProblematicFields {
                relation_fields,
                unknown_fields,
            }) => {
                if !unknown_fields.is_empty() {
                    let model_name = ctx.ast[relation_field.referenced_model].name();

                    let field_names = unknown_fields
                        .into_iter()
                        .map(|(_, field_name)| field_name)
                        .collect::<Vec<_>>()
                        .join(", ");

                    let msg = format!(
                        "The argument `references` must refer only to existing fields in the related model `{model_name}`. The following fields do not exist in the related model: {field_names}",
                    );

                    ctx.push_error(DatamodelError::new_validation_error(msg, attr.span));
                }

                if !relation_fields.is_empty() {
                    let msg = format!(
                        "The argument `references` must refer only to scalar fields in the related model `{}`. But it is referencing the following relation fields: {}",
                        ctx.ast[relation_field.referenced_model].name(),
                        relation_fields.iter().map(|(f, _)| f.name()).collect::<Vec<_>>().join(", "),
                    );
                    ctx.push_error(DatamodelError::new_validation_error(msg, attr.span));
                }

                Vec::new()
            }
        };

        relation_field.references = Some(references);
    }

    // Validate the `name` argument if present.
    match ctx.visit_default_arg("name").map(|arg| arg.as_str()).ok() {
        Some(Ok("")) => ctx.push_attribute_validation_error("A relation cannot have an empty name."),
        Some(Ok(name)) => {
            let interned_name = ctx.interner.intern(name);
            relation_field.name = Some(interned_name);
        }
        Some(Err(err)) => ctx.push_error(err),
        None => (),
    }

    // Validate referential actions.
    if let Some(on_delete) = ctx.visit_optional_arg("onDelete") {
        match on_delete.as_referential_action() {
            Ok(action) => {
                relation_field.on_delete = Some((action, on_delete.span()));
            }
            Err(err) => ctx.push_error(err),
        }
    }

    if let Some(on_update) = ctx.visit_optional_arg("onUpdate") {
        match on_update.as_referential_action() {
            Ok(action) => {
                relation_field.on_update = Some((action, on_update.span()));
            }
            Err(err) => ctx.push_error(err),
        }
    }

    let fk_name = {
        let mapped_name = match ctx.visit_optional_arg("map").map(|name| name.as_str()) {
            Some(Ok("")) => {
                ctx.push_attribute_validation_error("The `map` argument cannot be an empty string.");
                None
            }
            Some(Ok(name)) => Some(ctx.interner.intern(name)),
            Some(Err(err)) => {
                ctx.push_error(err);
                None
            }
            None => None,
        };

        mapped_name
    };

    relation_field.mapped_name = fk_name;
}

#[derive(Debug)]
enum FieldResolutionError<'ast> {
    AlreadyDealtWith,
    ProblematicFields {
        /// Fields that do not exist on the model.
        unknown_fields: Vec<(ast::TopId, &'ast str)>,
        /// Fields that exist on the model but are relation fields.
        relation_fields: Vec<(&'ast ast::Field, ast::FieldId)>,
    },
}

/// Takes an attribute argument, validates it as an array of constants, then
/// resolves  the constant as field names on the model. The error variant
/// contains the fields that are not in the model.
fn resolve_field_array_without_args<'db>(
    values: &ValueValidator<'db>,
    attribute_span: ast::Span,
    model_id: ast::ModelId,
    ctx: &mut Context<'db>,
) -> Result<Vec<ast::FieldId>, FieldResolutionError<'db>> {
    let constant_array = match values.as_constant_array() {
        Ok(values) => values,
        Err(err) => {
            ctx.push_error(err);
            return Err(FieldResolutionError::AlreadyDealtWith);
        }
    };

    let mut field_ids = Vec::with_capacity(constant_array.len());
    let mut unknown_fields = Vec::new();
    let mut relation_fields = Vec::new();
    let ast_model = &ctx.ast[model_id];

    for field_name in constant_array {
        if field_name.contains('.') {
            unknown_fields.push((ast::TopId::Model(model_id), field_name));
            continue;
        }

        // Does the field exist?
        let field_id = if let Some(field_id) = ctx.find_model_field(model_id, field_name) {
            field_id
        } else {
            unknown_fields.push((ast::TopId::Model(model_id), field_name));
            continue;
        };

        // Is the field a scalar field?
        if !ctx.types.scalar_fields.contains_key(&(model_id, field_id)) {
            relation_fields.push((&ctx.ast[model_id][field_id], field_id));
            continue;
        }

        // Is the field used twice?
        if field_ids.contains(&field_id) {
            ctx.push_error(DatamodelError::new_model_validation_error(
                &format!(
                    "The unique index definition refers to the field {} multiple times.",
                    ast_model[field_id].name()
                ),
                ast_model.name(),
                attribute_span,
            ));
            return Err(FieldResolutionError::AlreadyDealtWith);
        }

        field_ids.push(field_id);
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
    values: &ValueValidator<'db>,
    attribute_span: ast::Span,
    model_id: ast::ModelId,
    resolving: FieldResolvingSetup,
    ctx: &mut Context<'db>,
) -> Result<Vec<FieldWithArgs>, FieldResolutionError<'db>> {
    let constant_array = match values.as_field_array_with_args() {
        Ok(values) => values,
        Err(err) => {
            ctx.push_error(err);
            return Err(FieldResolutionError::AlreadyDealtWith);
        }
    };

    let mut field_ids = Vec::with_capacity(constant_array.len());
    let mut unknown_fields = Vec::new();
    let mut relation_fields = Vec::new();

    let ast_model = &ctx.ast[model_id];

    'fields: for (field_name, _, _) in &constant_array {
        let path = if field_name.contains('.') {
            if !resolving.follow_composites() {
                unknown_fields.push((ast::TopId::Model(model_id), *field_name));
                continue 'fields;
            }

            let field_count = field_name.split('.').count();
            let mut path_in_schema = field_name.split('.').enumerate();

            let (mut path, mut next_type) = match path_in_schema.next() {
                Some((_, field_shard)) => {
                    let field_id = match ctx.find_model_field(model_id, field_shard) {
                        Some(field_id) => field_id,
                        None => {
                            unknown_fields.push((ast::TopId::Model(model_id), field_shard));
                            continue 'fields;
                        }
                    };

                    if !ctx.types.scalar_fields.contains_key(&(model_id, field_id)) {
                        relation_fields.push((&ctx.ast[model_id][field_id], field_id));
                        continue 'fields;
                    }

                    match &ctx.types.scalar_fields[&(model_id, field_id)].r#type {
                        ScalarFieldType::CompositeType(ctid) => (IndexFieldPath::new(field_id), *ctid),
                        _ => {
                            unknown_fields.push((ast::TopId::Model(model_id), field_name));
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
                let field_id = match ctx.find_composite_type_field(next_type, field_shard) {
                    Some(field_id) => field_id,
                    None => {
                        unknown_fields.push((ast::TopId::CompositeType(next_type), field_shard));
                        continue 'fields;
                    }
                };

                path.push_field(next_type, field_id);

                match &ctx.types.composite_type_fields[&(next_type, field_id)].r#type {
                    ScalarFieldType::CompositeType(ctid) => {
                        next_type = *ctid;
                    }
                    _ if i < field_count - 1 => {
                        unknown_fields.push((ast::TopId::Model(model_id), field_name));
                        continue 'fields;
                    }
                    _ => (),
                }
            }

            path
        } else if let Some(field_id) = ctx.find_model_field(model_id, field_name) {
            // Is the field a scalar field?
            if !ctx.types.scalar_fields.contains_key(&(model_id, field_id)) {
                relation_fields.push((&ctx.ast[model_id][field_id], field_id));
                continue;
            } else {
                IndexFieldPath::new(field_id)
            }
        } else {
            unknown_fields.push((ast::TopId::Model(model_id), field_name));
            continue;
        };

        // Is the field used twice?
        if field_ids.contains(&path) {
            let path_str = match path.type_holding_the_indexed_field() {
                None => Cow::from(*field_name),
                Some(ctid) => {
                    let field_id = path.field_in_index();
                    let field_name = &ctx.ast[ctid][field_id].name.name;
                    let composite_type = &ctx.ast[ctid].name.name;

                    Cow::from(format!("{field_name} in type {composite_type}"))
                }
            };

            ctx.push_error(DatamodelError::new_model_validation_error(
                &format!("The unique index definition refers to the field {path_str} multiple times.",),
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
            .map(|((_, sort_order, length), field_location)| FieldWithArgs {
                path: field_location,
                sort_order,
                length,
            })
            .collect();

        Ok(fields_with_args)
    }
}

fn get_name_argument(ctx: &mut Context<'_>) -> Option<StringId> {
    match ctx.visit_optional_arg("name").map(|name| name.as_str()) {
        Some(Ok("")) => {
            ctx.push_attribute_validation_error("The `name` argument cannot be an empty string.");
        }
        Some(Err(err)) => ctx.push_error(err),
        Some(Ok(name)) => return Some(ctx.interner.intern(name)),
        None => (),
    }

    None
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
            "The `name` property within the `{}` attribute only allows for the following characters: `_a-zA-Z0-9`.",
            attribute
        ),
        object_name,
        span,
    ))
}

fn validate_clustering_setting(ctx: &mut Context<'_>) -> Option<bool> {
    match ctx
        .visit_optional_arg("clustered")
        .map(|sort| sort.as_constant_literal())
    {
        Some(Ok("true")) => Some(true),
        Some(Ok("false")) => Some(false),
        Some(Ok(other)) => {
            ctx.push_attribute_validation_error(&format!("Unknown boolean value: {}.", other));
            None
        }
        Some(Err(err)) => {
            ctx.push_error(err);
            None
        }
        None => None,
    }
}

mod default;
mod id;
mod map;
mod native_types;

use crate::{
    ast::{self, WithName, WithSpan},
    context::{Arguments, Context},
    types::{
        EnumAttributes, FieldWithArgs, IndexAlgorithm, IndexAttribute, IndexType, ModelAttributes, RelationField,
        ScalarField, ScalarFieldType, SortOrder,
    },
    AstString, DatamodelError, ValueValidator,
};
use diagnostics::Span;

pub(super) fn resolve_attributes(ctx: &mut Context<'_>) {
    for top in ctx.db.ast.iter_tops() {
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

fn resolve_composite_type_attributes<'ast>(
    ctid: ast::CompositeTypeId,
    ct: &'ast ast::CompositeType,
    ctx: &mut Context<'ast>,
) {
    for (field_id, field) in ct.iter_fields() {
        let mut ctfield = ctx.db.types.composite_type_fields.remove(&(ctid, field_id)).unwrap();

        ctx.visit_attributes(&field.attributes, |attributes, ctx| {
            // @map
            attributes.visit_optional_single("map", ctx, |map_args, ctx| {
                map::composite_type_field(ct, field, ctid, field_id, &mut ctfield, map_args, ctx);
            });

            // @default
            attributes.visit_optional_single("default", ctx, |args, ctx| {
                default::visit_composite_field_default(args, &mut ctfield, ctid, field_id, ctx);
            });
        });

        ctx.db.types.composite_type_fields.insert((ctid, field_id), ctfield);
    }
}

fn resolve_enum_attributes<'ast>(enum_id: ast::EnumId, ast_enum: &'ast ast::Enum, ctx: &mut Context<'ast>) {
    let mut enum_attributes = EnumAttributes::default();

    for (field_idx, field) in ast_enum.values.iter().enumerate() {
        ctx.visit_attributes(&field.attributes, |attributes, ctx| {
            // @map
            attributes.visit_optional_single("map", ctx, |map_args, ctx| {
                if let Some(mapped_name) = map::visit_map_attribute(map_args, ctx) {
                    enum_attributes.mapped_values.insert(field_idx as u32, mapped_name);
                    ctx.mapped_enum_value_names
                        .insert((enum_id, mapped_name), field_idx as u32);
                }
            })
        });
    }

    ctx.visit_attributes(&ast_enum.attributes, |attributes, ctx| {
        // @@map
        attributes.visit_optional_single("map", ctx, |map_args, ctx| {
            if let Some(mapped_name) = map::visit_map_attribute(map_args, ctx) {
                enum_attributes.mapped_name = Some(mapped_name);
                ctx.mapped_enum_names.insert(mapped_name, enum_id);
            }
        })
    });

    ctx.db.types.enum_attributes.insert(enum_id, enum_attributes);
}

fn resolve_model_attributes<'ast>(model_id: ast::ModelId, ast_model: &'ast ast::Model, ctx: &mut Context<'ast>) {
    let mut model_attributes = ModelAttributes::default();

    // First resolve all the attributes defined on fields **in isolation**.
    for (field_id, ast_field) in ast_model.iter_fields() {
        if let Some(mut scalar_field) = ctx.db.types.take_scalar_field(model_id, field_id) {
            visit_scalar_field_attributes(
                model_id,
                field_id,
                ast_model,
                ast_field,
                &mut model_attributes,
                &mut scalar_field,
                ctx,
            );

            ctx.db.types.scalar_fields.insert((model_id, field_id), scalar_field);
        } else if let Some(mut rf) = ctx.db.types.take_relation_field(model_id, field_id) {
            visit_relation_field_attributes(model_id, ast_field, &mut rf, ctx);
            ctx.db.types.relation_fields.insert((model_id, field_id), rf);
        } else {
            unreachable!(
                "{}.{} is neither a scalar field nor a relation field",
                ast_model.name(),
                ast_field.name()
            )
        }
    }

    // First resolve all the attributes defined on the model itself **in isolation**.
    ctx.visit_attributes(&ast_model.attributes, |attributes, ctx| {
        // @@ignore
        attributes.visit_optional_single("ignore", ctx, |_, ctx| {
            visit_model_ignore(model_id, &mut model_attributes, ctx);
        });

        // @@id
        attributes.visit_optional_single("id", ctx, |id_args, ctx| {
            id::model(id_args, &mut model_attributes, model_id, ctx);
        });

        // @@map
        attributes.visit_optional_single("map", ctx, |map_args, ctx| {
            map::model(&mut model_attributes, model_id, map_args, ctx)
        });

        // @@index
        attributes.visit_repeated("index", ctx, |args, ctx| {
            model_index(args, &mut model_attributes, model_id, ctx);
        });

        // @@unique
        attributes.visit_repeated("unique", ctx, |args, ctx| {
            model_unique(args, &mut model_attributes, model_id, ctx);
        });

        // @@fulltext
        attributes.visit_repeated("fulltext", ctx, |args, ctx| {
            model_fulltext(args, &mut model_attributes, model_id, ctx);
        });
    });

    // Model-global validations
    id::validate_id_field_arities(model_id, &model_attributes, ctx);

    ctx.db.types.model_attributes.insert(model_id, model_attributes);
}

fn visit_scalar_field_attributes<'ast>(
    model_id: ast::ModelId,
    field_id: ast::FieldId,
    ast_model: &'ast ast::Model,
    ast_field: &'ast ast::Field,
    model_attributes: &mut ModelAttributes<'ast>,
    scalar_field_data: &mut ScalarField<'ast>,
    ctx: &mut Context<'ast>,
) {
    ctx.visit_scalar_field_attributes(model_id, field_id, scalar_field_data.r#type, |attributes, ctx| {
        // @map
        attributes.visit_optional_single("map", ctx, |map_args, ctx| {
            map::scalar_field(ast_model, ast_field, model_id, field_id, scalar_field_data, map_args, ctx)
        });

        // @ignore
        attributes.visit_optional_single("ignore", ctx, |args, ctx| {
            if matches!(scalar_field_data.r#type, ScalarFieldType::Unsupported) {
                ctx.push_error(args.new_attribute_validation_error("Fields of type `Unsupported` cannot take an `@ignore` attribute. They are already treated as ignored by the client due to their type."));
            } else {
                scalar_field_data.is_ignored = true;
            }
        });

        // @relation
        attributes.visit_optional_single("relation", ctx, |args, ctx| {
            ctx.push_error(args.new_attribute_validation_error("Invalid field type, not a relation."));
        });

        // @id
        attributes.visit_optional_single("id", ctx, |args, ctx| {
            id::field(ast_model, field_id, model_attributes, args, ctx)
        });

        // @updatedAt
        attributes.visit_optional_single("updatedAt", ctx, |args, ctx| {
            if !matches!(scalar_field_data.r#type, ScalarFieldType::BuiltInScalar(crate::ScalarType::DateTime)) {
                ctx.push_error(args.new_attribute_validation_error(
                    "Fields that are marked with @updatedAt must be of type DateTime." ));

            }

            if ast_field.arity.is_list() {
                ctx.push_error(args.new_attribute_validation_error("Fields that are marked with @updatedAt cannot be lists."));
            }

            scalar_field_data.is_updated_at = true;
        });

        // @default
        attributes.visit_optional_single("default", ctx, |args, ctx| {
            default::visit_model_field_default(args, scalar_field_data, model_id, field_id, ctx);
        });

        if let ScalarFieldType::BuiltInScalar(_scalar_type) = scalar_field_data.r#type {
            // native type attributes
            attributes.visit_datasource_scoped(ctx, |datasource_name, type_name, args, _ctx| {
                native_types::visit_native_type_attribute(datasource_name, type_name, args, scalar_field_data)
            });
        }

        // @unique
        attributes.visit_optional_single("unique", ctx, |args, ctx| {
            visit_field_unique(field_id, model_attributes, args, ctx)
        });
    });
}

fn visit_field_unique<'ast>(
    field_id: ast::FieldId,
    model_attributes: &mut ModelAttributes<'ast>,
    args: &mut Arguments<'ast>,
    ctx: &mut Context<'ast>,
) {
    let mapped_name = match args.optional_arg("map").map(|name| name.as_str()) {
        Some(Ok("")) => {
            ctx.push_error(args.new_attribute_validation_error("The `map` argument cannot be an empty string."));
            None
        }
        Some(Ok(name)) => Some(name),
        Some(Err(err)) => {
            ctx.push_error(err);
            None
        }
        None => None,
    };

    let length = match args.optional_arg("length").map(|length| length.as_int()) {
        Some(Ok(length)) => Some(length as u32),
        Some(Err(err)) => {
            ctx.push_error(err);
            None
        }
        None => None,
    };

    let sort_order = match args.optional_arg("sort").map(|sort| sort.as_constant_literal()) {
        Some(Ok("Desc")) => Some(SortOrder::Desc),
        Some(Ok("Asc")) => Some(SortOrder::Asc),
        Some(Ok(other)) => {
            ctx.push_error(args.new_attribute_validation_error(&format!(
                "The `sort` argument can only be `Asc` or `Desc` you provided: {}.",
                other
            )));
            None
        }
        Some(Err(err)) => {
            ctx.push_error(err);
            None
        }
        None => None,
    };

    model_attributes.ast_indexes.push((
        args.attribute(),
        IndexAttribute {
            r#type: IndexType::Unique,
            fields: vec![FieldWithArgs {
                field_id,
                sort_order,
                length,
            }],
            source_field: Some(field_id),
            mapped_name,
            ..Default::default()
        },
    ))
}

fn visit_relation_field_attributes<'ast>(
    model_id: ast::ModelId,
    ast_field: &'ast ast::Field,
    relation_field: &mut RelationField<'ast>,
    ctx: &mut Context<'ast>,
) {
    ctx.visit_attributes(&ast_field.attributes, |attributes, ctx| {
        // @relation
        // Relation attributes are not required at this stage.
        attributes.visit_optional_single("relation", ctx, |relation_args, ctx| {
            visit_relation(relation_args, model_id, relation_field, ctx)
        });

        // @id
        attributes.visit_optional_single("id", ctx, |args, ctx| {
            ctx.push_error(args.new_attribute_validation_error(
                &format!(
                    "The field `{}` is a relation field and cannot be marked with `@id`. Only scalar fields can be declared as id.",
                    &ast_field.name(),
                ),
            ))
        });

        // @ignore
        attributes.visit_optional_single("ignore", ctx, |_, _| {
            relation_field.is_ignored = true;
        });

        // @default
        attributes.visit_optional_single("default", ctx, |args, ctx| {
            ctx.push_error(args.new_attribute_validation_error("Cannot set a default value on a relation field.", ));
            args.default_arg("value").ok();
        });

        // @map
        attributes.visit_optional_single("map", ctx, |args, ctx| {
            ctx.push_error(args.new_attribute_validation_error(
                "The attribute `@map` cannot be used on relation fields.",
            ));

            if let Err(err) = args.default_arg("name") {
                ctx.push_error(err)
            }
        });

        // @unique
        attributes.visit_optional_single("unique", ctx, |args, ctx| {
            let mut suggested_fields = Vec::new();

            for underlying_field in relation_field.fields.iter().flatten() {
                suggested_fields.push(ctx.db.ast[model_id][*underlying_field].name());
            }

            let suggestion = match suggested_fields.len() {
                0 => String::new(),
                1 =>
                    format!(
                        " Did you mean to put it on `{field}`?",
                        field = suggested_fields[0],
                    ),
                _ => {
                    format!(" Did you mean to provide `@@unique([{}])`?", field = suggested_fields.join(", "))
                }
            };

            ctx.push_error(args.new_attribute_validation_error(
                &format!(
                    "The field `{relation_field_name}` is a relation field and cannot be marked with `unique`. Only scalar fields can be made unique.{suggestion}",
                    relation_field_name = ast_field.name(),
                    suggestion = suggestion
                ),
            ));
        });
    });
}

fn visit_model_ignore(model_id: ast::ModelId, model_data: &mut ModelAttributes<'_>, ctx: &mut Context<'_>) {
    let ignored_field_errors: Vec<_> = ctx
        .db
        .types
        .range_model_scalar_fields(model_id)
        .filter(|(_, sf)| sf.is_ignored)
        .map(|(field_id, _)| {
            DatamodelError::new_attribute_validation_error(
                "Fields on an already ignored Model do not need an `@ignore` annotation.",
                "ignore",
                *ctx.db.ast[model_id][field_id].span(),
            )
        })
        .collect();

    for error in ignored_field_errors {
        ctx.push_error(error)
    }

    model_data.is_ignored = true;
}

/// Validate @@fulltext on models
fn model_fulltext<'ast>(
    args: &mut Arguments<'ast>,
    data: &mut ModelAttributes<'ast>,
    model_id: ast::ModelId,
    ctx: &mut Context<'ast>,
) {
    let mut index_attribute = IndexAttribute {
        r#type: IndexType::Fulltext,
        ..Default::default()
    };

    common_index_validations(args, &mut index_attribute, model_id, ctx);
    let mapped_name = match args.optional_arg("map").map(|name| name.as_str()) {
        Some(Ok("")) => {
            ctx.push_error(args.new_attribute_validation_error("The `map` argument cannot be an empty string."));
            None
        }
        Some(Ok(name)) => Some(name),
        Some(Err(err)) => {
            ctx.push_error(err);
            None
        }
        None => None,
    };

    index_attribute.mapped_name = mapped_name;

    data.ast_indexes.push((args.attribute(), index_attribute));
}

/// Validate @@index on models.
fn model_index<'ast>(
    args: &mut Arguments<'ast>,
    data: &mut ModelAttributes<'ast>,
    model_id: ast::ModelId,
    ctx: &mut Context<'ast>,
) {
    let mut index_attribute = IndexAttribute {
        r#type: IndexType::Normal,
        ..Default::default()
    };

    common_index_validations(args, &mut index_attribute, model_id, ctx);
    let name = get_name_argument(args, ctx);

    let mapped_name = match args.optional_arg("map").map(|name| name.as_str()) {
        Some(Ok("")) => {
            ctx.push_error(args.new_attribute_validation_error("The `map` argument cannot be an empty string."));
            None
        }
        Some(Ok(name)) => Some(name),
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
            let error = args.new_attribute_validation_error("The `@@index` attribute accepts the `name` argument as an alias for the `map` argument for legacy reasons. It does not accept both though. Please use the `map` argument to specify the database name of the index.");
            ctx.push_error(error);
            None
        }
        // backwards compatibility, accept name arg on normal indexes and use it as map arg.
        (Some(name), None) => Some(name),
        (None, Some(map)) => Some(map),
        (None, None) => None,
    };

    index_attribute.algorithm = match args.optional_arg("type").map(|sort| sort.as_constant_literal()) {
        Some(Ok("BTree")) => Some(IndexAlgorithm::BTree),
        Some(Ok("Hash")) => Some(IndexAlgorithm::Hash),
        Some(Ok(other)) => {
            ctx.push_error(args.new_attribute_validation_error(&format!(
                "The `type` argument can only be `BTree` or `Hash` you provided: {}.",
                other
            )));
            None
        }
        Some(Err(err)) => {
            ctx.push_error(err);
            None
        }
        None => None,
    };

    data.ast_indexes.push((args.attribute(), index_attribute));
}

/// Validate @@unique on models.
fn model_unique<'ast>(
    args: &mut Arguments<'ast>,
    data: &mut ModelAttributes<'ast>,
    model_id: ast::ModelId,
    ctx: &mut Context<'ast>,
) {
    let mut index_attribute = IndexAttribute {
        r#type: IndexType::Unique,
        ..Default::default()
    };
    common_index_validations(args, &mut index_attribute, model_id, ctx);

    let ast_model = &ctx.db.ast()[model_id];
    let name = get_name_argument(args, ctx);

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
        let mapped_name = match args.optional_arg("map").map(|name| name.as_str()) {
            Some(Ok("")) => {
                ctx.push_error(args.new_attribute_validation_error("The `map` argument cannot be an empty string."));
                None
            }
            Some(Ok(name)) => Some(name),
            Some(Err(err)) => {
                ctx.push_error(err);
                None
            }
            None => None,
        };

        if let Some(name) = name {
            validate_client_name(args.span(), &ast_model.name.name, name, "@@unique", ctx);
        }

        mapped_name
    };

    index_attribute.name = name;
    index_attribute.mapped_name = mapped_name;

    data.ast_indexes.push((args.attribute(), index_attribute));
}

fn common_index_validations<'ast>(
    args: &mut Arguments<'ast>,
    index_data: &mut IndexAttribute<'ast>,
    model_id: ast::ModelId,
    ctx: &mut Context<'ast>,
) {
    let fields = match args.default_arg("fields") {
        Ok(fields) => fields,
        Err(err) => {
            return ctx.push_error(err);
        }
    };

    match resolve_field_array_with_args(&fields, args.span(), model_id, ctx) {
        Ok(fields) => {
            index_data.fields = fields;
        }
        Err(FieldResolutionError::AlreadyDealtWith) => (),
        Err(FieldResolutionError::ProblematicFields {
            unknown_fields: unresolvable_fields,
            relation_fields,
        }) => {
            if !unresolvable_fields.is_empty() {
                ctx.push_error({
                    let message: &str = &format!(
                        "The {}index definition refers to the unknown fields {}.",
                        if index_data.is_unique() { "unique " } else { "" },
                        unresolvable_fields.join(", "),
                    );
                    let model_name = ctx.db.ast()[model_id].name();
                    let span = args.span();
                    DatamodelError::ModelValidationError {
                        message: String::from(message),
                        model_name: String::from(model_name),
                        span,
                    }
                });
            }

            if !relation_fields.is_empty() {
                let mut suggested_fields = Vec::new();

                for (_, field_id) in &relation_fields {
                    let relation_field = &ctx.db.types.relation_fields[&(model_id, *field_id)];
                    for underlying_field in relation_field.fields.iter().flatten() {
                        suggested_fields.push(ctx.db.ast[model_id][*underlying_field].name());
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
                    ctx.db.ast[model_id].name(),
                    args.span(),
                ));
            }
        }
    }
}

/// @relation validation for relation fields.
fn visit_relation<'ast>(
    args: &mut Arguments<'ast>,
    model_id: ast::ModelId,
    relation_field: &mut RelationField<'ast>,
    ctx: &mut Context<'ast>,
) {
    relation_field.relation_attribute = Some(args.attribute());

    if let Some(fields) = args.optional_arg("fields") {
        let fields = match resolve_field_array_without_args(&fields, args.span(), model_id, ctx) {
            Ok(fields) => fields,
            Err(FieldResolutionError::AlreadyDealtWith) => Vec::new(),
            Err(FieldResolutionError::ProblematicFields {
                unknown_fields: unresolvable_fields,
                relation_fields,
            }) => {
                if !unresolvable_fields.is_empty() {
                    ctx.push_error(DatamodelError::new_validation_error(format!("The argument fields must refer only to existing fields. The following fields do not exist in this model: {}", unresolvable_fields.join(", ")), fields.span()))
                }

                if !relation_fields.is_empty() {
                    ctx.push_error(DatamodelError::new_validation_error(format!("The argument fields must refer only to scalar fields. But it is referencing the following relation fields: {}", relation_fields.iter().map(|(f, _)| f.name()).collect::<Vec<_>>().join(", ")), fields.span()));
                }

                Vec::new()
            }
        };

        relation_field.fields = Some(fields);
    }

    if let Some(references) = args.optional_arg("references") {
        let references = match resolve_field_array_without_args(
            &references,
            args.span(),
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
                    let msg = format!(

                        "The argument `references` must refer only to existing fields in the related model `{}`. The following fields do not exist in the related model: {}",
                        ctx.db.ast[relation_field.referenced_model].name(),
                        unknown_fields.join(", "),
                    );
                    ctx.push_error(DatamodelError::new_validation_error(msg, args.span()));
                }

                if !relation_fields.is_empty() {
                    let msg = format!(
                        "The argument `references` must refer only to scalar fields in the related model `{}`. But it is referencing the following relation fields: {}",
                        ctx.db.ast[relation_field.referenced_model].name(),
                        relation_fields.iter().map(|(f, _)| f.name()).collect::<Vec<_>>().join(", "),
                    );
                    ctx.push_error(DatamodelError::new_validation_error(msg, args.span()));
                }

                Vec::new()
            }
        };

        relation_field.references = Some(references);
    }

    // Validate the `name` argument if present.
    match args.optional_default_arg("name").map(|arg| arg.as_str_with_span()) {
        Some(Ok(("", _))) => {
            ctx.push_error(args.new_attribute_validation_error("A relation cannot have an empty name."))
        }
        Some(Ok((name, span))) => {
            relation_field.name = Some(AstString::from_literal(name.to_owned(), span));
        }
        Some(Err(err)) => ctx.push_error(err),
        None => (),
    }

    // Validate referential actions.
    if let Some(on_delete) = args.optional_arg("onDelete") {
        match on_delete.as_referential_action() {
            Ok(action) => {
                relation_field.on_delete = Some((action, on_delete.span()));
            }
            Err(err) => ctx.push_error(err),
        }
    }

    if let Some(on_update) = args.optional_arg("onUpdate") {
        match on_update.as_referential_action() {
            Ok(action) => {
                relation_field.on_update = Some((action, on_update.span()));
            }
            Err(err) => ctx.push_error(err),
        }
    }

    let fk_name = {
        let mapped_name = match args.optional_arg("map").map(|name| name.as_str()) {
            Some(Ok("")) => {
                ctx.push_error(args.new_attribute_validation_error("The `map` argument cannot be an empty string."));
                None
            }
            Some(Ok(name)) => Some(name),
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

enum FieldResolutionError<'ast> {
    AlreadyDealtWith,
    ProblematicFields {
        /// Fields that do not exist on the model.
        unknown_fields: Vec<&'ast str>,
        /// Fields that exist on the model but are relation fields.
        relation_fields: Vec<(&'ast ast::Field, ast::FieldId)>,
    },
}

/// Takes an attribute argument, validates it as an array of constants, then
/// resolves  the constant as field names on the model. The error variant
/// contains the fields that are not in the model.
fn resolve_field_array_without_args<'ast>(
    values: &ValueValidator<'ast>,
    attribute_span: ast::Span,
    model_id: ast::ModelId,
    ctx: &mut Context<'ast>,
) -> Result<Vec<ast::FieldId>, FieldResolutionError<'ast>> {
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
    let ast_model = &ctx.db.ast[model_id];

    for field_name in constant_array {
        // Does the field exist?
        let field_id = if let Some(field_id) = ctx.db.find_model_field(model_id, field_name) {
            field_id
        } else {
            unknown_fields.push(field_name);
            continue;
        };

        // Is the field a scalar field?
        if !ctx.db.types.scalar_fields.contains_key(&(model_id, field_id)) {
            relation_fields.push((&ctx.db.ast[model_id][field_id], field_id));
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

/// Takes an attribute argument, validates it as an array of fields with potentially args,
/// then resolves  the constant literal as field names on the model. The error variant
/// contains the fields that are not in the model.
fn resolve_field_array_with_args<'ast>(
    values: &ValueValidator<'ast>,
    attribute_span: ast::Span,
    model_id: ast::ModelId,
    ctx: &mut Context<'ast>,
) -> Result<Vec<FieldWithArgs>, FieldResolutionError<'ast>> {
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
    let ast_model = &ctx.db.ast[model_id];

    for (field_name, _, _) in &constant_array {
        // Does the field exist?
        let field_id = if let Some(field_id) = ctx.db.find_model_field(model_id, field_name) {
            field_id
        } else {
            unknown_fields.push(*field_name);
            continue;
        };

        // Is the field a scalar field?
        if !ctx.db.types.scalar_fields.contains_key(&(model_id, field_id)) {
            relation_fields.push((&ctx.db.ast[model_id][field_id], field_id));
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
        let fields_with_args = constant_array
            .into_iter()
            .zip(field_ids)
            .map(|((_, sort_order, length), field_id)| FieldWithArgs {
                field_id,
                sort_order,
                length,
            })
            .collect();

        Ok(fields_with_args)
    }
}

fn get_name_argument<'ast>(args: &mut Arguments<'ast>, ctx: &mut Context<'ast>) -> Option<&'ast str> {
    match args.optional_arg("name").map(|name| name.as_str()) {
        Some(Ok("")) => {
            ctx.push_error(args.new_attribute_validation_error("The `name` argument cannot be an empty string."))
        }
        Some(Err(err)) => ctx.push_error(err),
        Some(Ok(name)) => return Some(name),
        None => (),
    }

    None
}

fn validate_client_name(span: Span, object_name: &str, name: &str, attribute: &str, ctx: &mut Context<'_>) {
    // only Alphanumeric characters and underscore are allowed due to this making its way into the client API
    // todo what about starting with a number or underscore?
    let is_valid = name
        .chars()
        .all(|c| c == '_' || c.is_ascii_digit() || c.is_ascii_alphabetic());

    if is_valid {
        return;
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

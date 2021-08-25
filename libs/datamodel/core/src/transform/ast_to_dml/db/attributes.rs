mod autoincrement;
mod native_types;
mod relation;

use super::{
    context::{Arguments, Context},
    types::{IndexData, ModelData, RelationField, ScalarField},
};
use crate::ast::{FieldId, Model, ModelId};
use crate::common::constraint_names::ConstraintNames;

use crate::{
    ast::{self, WithName},
    diagnostics::DatamodelError,
    dml,
    transform::{ast_to_dml::db::ScalarFieldType, helpers::ValueValidator},
    IndexType,
};

use crate::transform::ast_to_dml::db::types::PrimaryKeyData;
use itertools::Itertools;
use prisma_value::PrismaValue;
use std::borrow::Cow;
use std::collections::HashSet;

pub(super) fn resolve_model_and_field_attributes<'ast>(
    model_id: ast::ModelId,
    ast_model: &'ast ast::Model,
    ctx: &mut Context<'ast>,
) {
    let mut model_data = ctx
        .db
        .types
        .take_model_data(&model_id)
        .expect("Broken invariant: no ModelData for model in visit_model_attributes.");

    {
        // set database name already if possible without validating against other names / unused args / other errors
        let map = &ast_model.attributes.iter().find(|a| a.name.name == "map");
        if let Some(map_attribute) = map {
            let mapped_name = match (
                map_attribute.arguments.iter().find(|a| a.name() == "name"),
                map_attribute.arguments.iter().find(|a| a.name() == ""),
            ) {
                (Some(arg), None) => Some(ValueValidator::new(&arg.value).as_str().unwrap()),
                (None, Some(arg)) => Some(ValueValidator::new(&arg.value).as_str().unwrap()),
                _ => None,
            };

            model_data.mapped_name = mapped_name;
        }
    }

    //first go over all scalars since relations depend on them
    let mut seen_scalars = vec![];
    for (field_id, ast_field) in ast_model.iter_fields() {
        if let Some(mut scalar_field) = ctx.db.types.take_scalar_field(model_id, field_id) {
            seen_scalars.push((model_id, field_id));
            visit_scalar_field_attributes(
                model_id,
                field_id,
                ast_model,
                ast_field,
                &mut model_data,
                &mut scalar_field,
                ctx,
            );

            ctx.db.types.scalar_fields.insert((model_id, field_id), scalar_field);
        }
    }

    for (field_id, ast_field) in ast_model.iter_fields() {
        if let Some(mut rf) = ctx.db.types.take_relation_field(model_id, field_id) {
            visit_relation_field_attributes(model_id, field_id, ast_field, &model_data, &mut rf, ctx);
            ctx.db.types.relation_fields.insert((model_id, field_id), rf);
        } else if !seen_scalars.contains(&(model_id, field_id)) {
            unreachable!(
                "{}.{} is neither a scalar field nor a relation field",
                ast_model.name(),
                ast_field.name()
            )
        }
    }

    ctx.visit_attributes(&ast_model.attributes, |attributes, ctx| {
        // @@ignore
        attributes.visit_optional_single("ignore", ctx, |_, ctx| {
            visit_model_ignore(model_id, &mut model_data, ctx);
        });

        // @@id
        attributes.visit_optional_single("id", ctx, |id_args, ctx| {
            visit_model_id(id_args, &mut model_data, model_id, ctx);
        });

        // @@map
        attributes.visit_optional_single("map", ctx, |map_args, ctx| {
            let mapped_name = match visit_map_attribute(map_args, ctx) {
                Some(name) => name,
                None => return,
            };

            model_data.mapped_name = Some(mapped_name);

            if let Some(existing_model_id) = ctx.mapped_model_names.insert(mapped_name, model_id) {
                let existing_model_name = ctx.db.ast[existing_model_id].name();
                ctx.push_error(DatamodelError::new_duplicate_model_database_name_error(
                    mapped_name.to_owned(),
                    existing_model_name.to_owned(),
                    ast_model.span,
                ));
            }

            if let Some(existing_model_id) = ctx.db.names.tops.get(mapped_name).and_then(|id| id.as_model_id()) {
                let existing_model_name = ctx.db.ast[existing_model_id].name();
                ctx.push_error(DatamodelError::new_duplicate_model_database_name_error(
                    mapped_name.to_owned(),
                    existing_model_name.to_owned(),
                    map_args.span(),
                ));
            }
        });

        // @@index
        attributes.visit_repeated("index", ctx, |args, ctx| {
            model_index(args, &mut model_data, model_id, ctx);
        });

        // @@unique
        attributes.visit_repeated("unique", ctx, |args, ctx| {
            model_unique(args, &mut model_data, model_id, ctx);
        });
    });

    ctx.db.types.models.insert(model_id, model_data);

    autoincrement::validate_auto_increment(model_id, ctx);
}

pub(super) fn validate_index_names(ctx: &mut Context<'_>) {
    if ctx.db.active_connector().supports_multiple_indexes_with_same_name() {
        return;
    }

    let mut index_names = HashSet::new();
    let mut errors = Vec::new();

    for (attr, index) in ctx
        .db
        .ast
        .iter_models()
        .flat_map(|(model_id, _)| ctx.db.types.models[&model_id].indexes.iter())
    {
        let index_name = match &index.db_name {
            Some(index_name) => index_name.to_string(),
            None => continue,
        };

        if index_names.insert(index_name.clone()) {
            continue; // true means this name hasn't been seen before
        }

        errors.push(DatamodelError::new_multiple_indexes_with_same_name_are_not_supported(
            &index_name,
            attr.span,
        ))
    }

    errors.into_iter().for_each(|err| ctx.push_error(err))
}

fn visit_scalar_field_attributes<'ast>(
    model_id: ast::ModelId,
    field_id: ast::FieldId,
    ast_model: &'ast ast::Model,
    ast_field: &'ast ast::Field,
    model_data: &mut ModelData<'ast>,
    scalar_field_data: &mut ScalarField<'ast>,
    ctx: &mut Context<'ast>,
) {
    ctx.visit_scalar_field_attributes(model_id, field_id, scalar_field_data.r#type, |attributes, ctx| {
        // @map
         attributes.visit_optional_single("map", ctx, |map_args, ctx| {
             let mapped_name = match visit_map_attribute(map_args, ctx) {
                Some(name) => name,
                None => return
             };

            scalar_field_data.mapped_name = Some(mapped_name);

            if ctx.mapped_model_scalar_field_names.insert((model_id, mapped_name), field_id).is_some() {
                ctx.push_error(DatamodelError::new_duplicate_field_error(
                    ast_model.name(),
                    ast_field.name(),
                    ast_field.span,
                ));
            }

            if let Some(field_id) = ctx.db.names.model_fields.get(&(model_id, mapped_name)) {
                // @map only conflicts with _scalar_ fields
                if !ctx.db.types.scalar_fields.contains_key(&(model_id, *field_id)) {
                    return
                }
                ctx.push_error(DatamodelError::new_duplicate_field_error(
                    ast_model.name(),
                    ast_field.name(),
                    ast_field.span,
                ));
            }
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
            match model_data.primary_key {
                Some(_) => ctx.push_error(DatamodelError::new_model_validation_error(
                    "At most one field must be marked as the id field with the `@id` attribute.",
                    ast_model.name(),
                    ast_model.span,
                )),
                None => {
                    let db_name =
                        primary_key_constraint_name(ast_model, model_data, args,  "@id", ctx)
                    ;

                    model_data.primary_key = Some(PrimaryKeyData{
                        name: None,
                        db_name,
                        fields: vec![field_id],
                        source_field: Some(field_id)
                    })
                },
            }

            match (model_data.is_ignored, ast_field.arity) {
                (true, _) |
                (false, ast::FieldArity::Required) => (),
                (false, ast::FieldArity::List) |
                (false, ast::FieldArity::Optional) => ctx.push_error(args.new_attribute_validation_error("Fields that are marked as id must be required.")),
            }
        });

         // @updatedAt
         attributes.visit_optional_single("updatedAt", ctx, |args, ctx| {
             if !matches!(scalar_field_data.r#type, ScalarFieldType::BuiltInScalar(tpe) if tpe.is_datetime()) {
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
            visit_field_default(args, model_data, scalar_field_data, model_id, field_id, ctx);
         });

        if let ScalarFieldType::BuiltInScalar(scalar_type) = scalar_field_data.r#type {
            // native type attributes
            attributes.visit_datasource_scoped(ctx, |type_name, args, ctx| {
                native_types::visit_native_type_attribute(type_name, args, scalar_type, scalar_field_data, ctx)
            });
        }

         // @unique
         attributes.visit_optional_single("unique", ctx, |args, ctx| {

             let model_db_name = model_data.mapped_name.unwrap_or(&ast_model.name.name);
             let field_db_name = scalar_field_data.mapped_name.unwrap_or(&ast_field.name.name);

             let db_name : Option<Cow<'ast, str>> = {

                 let generated_name = ConstraintNames::index_name(model_db_name, &[field_db_name], IndexType::Unique, ctx.db.active_connector());
                 let db_name = get_map_argument(args,  generated_name,ctx);
                 validate_db_name(ast_model, args, &db_name, "@unique", ctx);

                 db_name

             };

            model_data.indexes.push((args.attribute(), IndexData {
                is_unique: true,
                fields: vec![field_id],
                source_field: Some(field_id),
                name: None,
                db_name,
            }))
        });
    });
}

fn primary_key_constraint_name<'ast>(
    ast_model: &'ast Model,
    model_data: &mut ModelData<'ast>,
    args: &mut Arguments<'ast>,
    attribute: &'ast str,
    ctx: &mut Context<'ast>,
) -> Option<Cow<'ast, str>> {
    let generated_name = ConstraintNames::primary_key_name(
        model_data.mapped_name.unwrap_or(&ast_model.name.name),
        ctx.db.active_connector(),
    );

    let db_name = match args.optional_arg("map").map(|name| name.as_str()) {
        Some(Ok("")) => error_on_empty_string_cow(args, ctx),
        Some(Ok(name)) => Some(name.into()),
        Some(Err(err)) => push_error_cow(err, ctx),
        None if ctx.db.active_connector().supports_named_primary_keys() => Some(generated_name.into()),
        None => None,
    };

    validate_db_name(ast_model, args, &db_name, attribute, ctx);

    if db_name.is_some() && !ctx.db.active_connector().supports_named_primary_keys() {
        ctx.push_error(DatamodelError::new_model_validation_error(
            "You defined a database name for the primary key on the model. This is not supported by the provider.",
            &ast_model.name.name,
            ast_model.span,
        ));
    }
    db_name
}

fn default_value_constraint_name<'ast>(
    args: &mut Arguments<'ast>,
    ast_model: &'ast Model,
    model_data: &ModelData<'ast>,
    ast_field: &'ast ast::Field,
    field_data: &ScalarField<'ast>,
    ctx: &mut Context<'ast>,
) -> Option<String> {
    let db_name = match args.optional_arg("map").map(|name| name.as_str()) {
        Some(Ok("")) => error_on_empty_string_cow(args, ctx),
        Some(Ok(name)) => Some(name.into()),
        Some(Err(err)) => push_error_cow(err, ctx),
        None if ctx.db.active_connector().supports_named_default_values() => {
            let model_name = model_data.mapped_name.unwrap_or_else(|| ast_model.name());
            let field_name = field_data.mapped_name.unwrap_or_else(|| ast_field.name());

            let generated_name = ConstraintNames::default_name(model_name, field_name, ctx.db.active_connector());

            Some(generated_name.into())
        }
        None => None,
    };

    validate_db_name(ast_model, args, &db_name, "@default", ctx);

    if db_name.is_some() && !ctx.db.active_connector().supports_named_default_values() {
        ctx.push_error(args.new_attribute_validation_error(
            "You defined a database name for the default value of a field on the model. This is not supported by the provider.",
        ));
    }

    db_name.map(|cow| cow.to_string())
}

fn visit_relation_field_attributes<'ast>(
    model_id: ast::ModelId,
    field_id: ast::FieldId,
    ast_field: &'ast ast::Field,
    model_data: &ModelData<'ast>,
    relation_field: &mut RelationField<'ast>,
    ctx: &mut Context<'ast>,
) {
    ctx.visit_attributes(&ast_field.attributes, |attributes, ctx| {
        // @relation
        // Relation attributes are not required _yet_ at this stage. The schema has to be parseable for standardization.
        attributes.visit_optional_single("relation", ctx, |relation_args, ctx| {
            visit_relation(relation_args, model_id, field_id, model_data, relation_field, ctx)
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

/// @default on scalar fields
fn visit_field_default<'ast>(
    args: &mut Arguments<'ast>,
    model_data: &mut ModelData<'ast>,
    field_data: &mut ScalarField<'ast>,
    model_id: ast::ModelId,
    field_id: ast::FieldId,
    ctx: &mut Context<'ast>,
) {
    let value = match args.default_arg("value") {
        Ok(value) => value,
        Err(err) => return ctx.push_error(err),
    };

    let ast_model = &ctx.db.ast[model_id];
    let ast_field = &ast_model[field_id];
    if ast_field.arity.is_list() {
        return ctx.push_error(args.new_attribute_validation_error("Cannot set a default value on list field."));
    }

    // Error on `dbgenerated("")
    if let Some(generator) = value.as_value_generator().ok().filter(|val| val.is_dbgenerated()) {
        if generator.as_dbgenerated() == Some("") {
            ctx.push_error(args.new_attribute_validation_error(
                "dbgenerated() takes either no argument, or a single nonempty string argument.",
            ));
        }
    }

    // Resolve the default to a dml::DefaultValue. We must loop in order to
    // resolve type aliases.
    let mut r#type = field_data.r#type;

    loop {
        match r#type {
            ScalarFieldType::Enum(enum_id) => {
                match value.as_constant_literal() {
                    Ok(value) => {
                        if ctx.db.ast[enum_id].values.iter().any(|v| v.name() == value) {
                            let mut default = dml::DefaultValue::new_single(PrismaValue::Enum(value.to_owned()));

                            if let Some(name) =
                                default_value_constraint_name(args, ast_model, model_data, ast_field, field_data, ctx)
                            {
                                default.set_db_name(name);
                            }

                            field_data.default = Some(default);
                        } else {
                            ctx.push_error(args.new_attribute_validation_error(
                                "The defined default value is not a valid value of the enum specified for the field.",
                            ))
                        }
                    }
                    Err(err) => {
                        match value.as_value_generator() {
                            Ok(generator) if generator.is_dbgenerated() => {
                                let mut default = dml::DefaultValue::new_expression(generator);

                                if let Some(name) = default_value_constraint_name(
                                    args, ast_model, model_data, ast_field, field_data, ctx,
                                ) {
                                    default.set_db_name(name);
                                }

                                field_data.default = Some(default);
                            }
                            Ok(_) | Err(_) => ctx.push_error(args.new_attribute_validation_error(&err.to_string())),
                        };
                    }
                };
            }
            ScalarFieldType::BuiltInScalar(scalar_type) => {
                match value.as_default_value_for_scalar_type(scalar_type) {
                    Ok(mut default) => {
                        if args.has_arg("map") && default.is_autoincrement() {
                            ctx.push_error(args.new_attribute_validation_error(
                                "Naming an autoincrement default value is not allowed.",
                            ))
                        }

                        if let Some(name) =
                            default_value_constraint_name(args, ast_model, model_data, ast_field, field_data, ctx)
                        {
                            default.set_db_name(name);
                        }

                        field_data.default = Some(default);
                    }
                    Err(err) => ctx.push_error(args.new_attribute_validation_error(&err.to_string())),
                }
            }
            ScalarFieldType::Alias(alias_id) => {
                r#type = ctx.db.types.type_aliases[&alias_id];
                continue;
            }
            ScalarFieldType::Unsupported => {
                match value.as_value_generator() {
                    Ok(generator) if generator.is_dbgenerated() => {
                        field_data.default = Some(dml::DefaultValue::new_expression(generator))
                    }
                    Ok(_) => ctx.push_error(args.new_attribute_validation_error(
                        "Only @default(dbgenerated()) can be used for Unsupported types.",
                    )),
                    Err(err) => ctx.push_error(args.new_attribute_validation_error(&err.to_string())),
                }
            }
        }

        break;
    }
}

/// @@id on models
fn visit_model_id<'ast>(
    args: &mut Arguments<'ast>,
    model_data: &mut ModelData<'ast>,
    model_id: ast::ModelId,
    ctx: &mut Context<'ast>,
) {
    let fields = match args.default_arg("fields") {
        Ok(value) => value,
        Err(err) => return ctx.push_error(err),
    };

    if !ctx.db.active_connector().supports_compound_ids() {
        return ctx.push_error(DatamodelError::new_model_validation_error(
            "The current connector does not support compound ids.",
            ctx.db.ast[model_id].name(),
            args.span(),
        ));
    }

    let resolved_fields = match resolve_field_array(&fields, args.span(), model_id, ctx) {
        Ok(fields) => fields,
        Err(FieldResolutionError::AlreadyDealtWith) => return,
        Err(FieldResolutionError::ProblematicFields {
            unknown_fields: unresolvable_fields,
            relation_fields,
        }) => {
            if !unresolvable_fields.is_empty() {
                ctx.push_error(DatamodelError::new_model_validation_error(
                    &format!(
                        "The multi field id declaration refers to the unknown fields {}.",
                        unresolvable_fields.join(", "),
                    ),
                    ctx.db.ast[model_id].name(),
                    fields.span(),
                ));
            }

            if !relation_fields.is_empty() {
                ctx.push_error(DatamodelError::new_model_validation_error(&format!("The id definition refers to the relation fields {}. ID definitions must reference only scalar fields.", relation_fields.iter().map(|(f, _)| f.name()).join(", ")), ctx.db.ast[model_id].name(), args.span()));
            }

            return;
        }
    };

    let ast_model = &ctx.db.ast[model_id];

    // ID attribute fields must reference only required fields.
    let fields_that_are_not_required: Vec<&str> = resolved_fields
        .iter()
        .map(|field_id| &ctx.db.ast[model_id][*field_id])
        .filter(|field| !field.arity.is_required())
        .map(|field| field.name())
        .collect();

    if !fields_that_are_not_required.is_empty() {
        ctx.push_error(DatamodelError::new_model_validation_error(
            &format!(
                "The id definition refers to the optional fields {}. ID definitions must reference only required fields.",
                fields_that_are_not_required.join(", ")
            ),
            &ast_model.name.name,
            args.span(),
        ))
    }

    if model_data.primary_key.is_some() {
        ctx.push_error(DatamodelError::new_model_validation_error(
            "Each model must have at most one id criteria. You can't have `@id` and `@@id` at the same time.",
            ast_model.name(),
            ast_model.span,
        ))
    }

    let (name, db_name) = {
        let db_name = primary_key_constraint_name(ast_model, model_data, args, "@@id", ctx);
        let name = get_name_argument(args, ctx);
        if let Some(err) = ConstraintNames::is_client_name_valid(args.span(), &ast_model.name.name, name, "@@id") {
            ctx.push_error(err);
        }

        (name, db_name)
    };

    model_data.primary_key = Some(PrimaryKeyData {
        name,
        db_name,
        fields: resolved_fields,
        source_field: None,
    });
}

fn visit_model_ignore(model_id: ast::ModelId, model_data: &mut ModelData<'_>, ctx: &mut Context<'_>) {
    let ignored_field_errors: Vec<_> = ctx
        .db
        .iter_model_scalar_fields(model_id)
        .filter(|(_, sf)| sf.is_ignored)
        .map(|(field_id, _)| {
            DatamodelError::new_attribute_validation_error(
                "Fields on an already ignored Model do not need an `@ignore` annotation.",
                "ignore",
                ctx.db.ast[model_id][field_id].span,
            )
        })
        .collect();

    for error in ignored_field_errors {
        ctx.push_error(error)
    }

    model_data.is_ignored = true;
}

/// Validate @@index on models.
fn model_index<'ast>(
    args: &mut Arguments<'ast>,
    data: &mut ModelData<'ast>,
    model_id: ast::ModelId,
    ctx: &mut Context<'ast>,
) {
    let mut index_data = IndexData {
        is_unique: false,
        ..Default::default()
    };

    common_index_validations(args, &mut index_data, model_id, ctx);
    let ast_model = &ctx.db.ast[model_id];
    let model_db_name = data.mapped_name.unwrap_or(&ast_model.name.name);

    let field_db_names = get_field_db_names(model_id, &index_data.fields, ctx);
    let name = get_name_argument(args, ctx);

    let generated_name = ConstraintNames::index_name(
        model_db_name,
        &field_db_names,
        IndexType::Normal,
        ctx.db.active_connector(),
    );

    let db_name = match args.optional_arg("map").map(|name| name.as_str()) {
        Some(Ok("")) => error_on_empty_string_cow(args, ctx),
        Some(Ok(name)) => Some(name.into()),
        Some(Err(err)) => push_error_cow(err, ctx),
        None => None,
    };

    validate_db_name(ast_model, args, &db_name, "@@index", ctx);

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
    index_data.db_name = match (name, db_name) {
        (Some(_), Some(_)) => {
            let error = args.new_attribute_validation_error("The `@@index` attribute accepts the `name` argument as an alias for the `map` argument for legacy reasons. It does not accept both though. Please use the `map` argument to specify the database name of the index.");
            ctx.push_error(error);
            None
        }
        // backwards compatibility, accept name arg on normal indexes and use it as map arg.
        (Some(name), None) => Some(name.into()),
        (None, Some(map)) => Some(map),
        (None, None) => Some(generated_name.into()),
    };

    data.indexes.push((args.attribute(), index_data));
}

/// Validate @@unique on models.
fn model_unique<'ast>(
    args: &mut Arguments<'ast>,
    data: &mut ModelData<'ast>,
    model_id: ast::ModelId,
    ctx: &mut Context<'ast>,
) {
    let mut index_data = IndexData {
        is_unique: true,
        ..Default::default()
    };
    common_index_validations(args, &mut index_data, model_id, ctx);

    let ast_model = &ctx.db.ast[model_id];
    let model_db_name = data.mapped_name.unwrap_or(&ast_model.name.name);

    let field_db_names: Vec<&str> = get_field_db_names(model_id, &index_data.fields, ctx);

    let name = get_name_argument(args, ctx);

    let (name, db_name) = {
        //We do not want to break existing datamodels for client purposes that use the old `@@unique([field], name: "ClientANDdbname")`
        //Since we still parse the name argument and pass it to the client they will keep working
        //Migrate will however get a new generated db name for the constraint in that case and try to change the underlying constraint name
        //We are fine with that since this is not automatically breaking but rather prompts a migration upon the first run on migrate
        //The client is unaffected by this.
        let generated_name = ConstraintNames::index_name(
            model_db_name,
            &field_db_names,
            IndexType::Unique,
            ctx.db.active_connector(),
        );

        let db_name = get_map_argument(args, generated_name, ctx);

        validate_db_name(ast_model, args, &db_name, "@@unique", ctx);

        if let Some(err) = ConstraintNames::is_client_name_valid(args.span(), &ast_model.name.name, name, "@@unique") {
            ctx.push_error(err);
        }

        (name, db_name)
    };

    index_data.name = name;
    index_data.db_name = db_name;

    data.indexes.push((args.attribute(), index_data));
}

fn common_index_validations<'ast>(
    args: &mut Arguments<'ast>,
    index_data: &mut IndexData<'ast>,
    model_id: ast::ModelId,
    ctx: &mut Context<'ast>,
) {
    let fields = match args.default_arg("fields") {
        Ok(fields) => fields,
        Err(err) => {
            return ctx.push_error(err);
        }
    };

    match resolve_field_array(&fields, args.span(), model_id, ctx) {
        Ok(fields) => {
            index_data.fields = fields;
        }
        Err(FieldResolutionError::AlreadyDealtWith) => (),
        Err(FieldResolutionError::ProblematicFields {
            unknown_fields: unresolvable_fields,
            relation_fields,
        }) => {
            if !unresolvable_fields.is_empty() {
                ctx.push_error(DatamodelError::new_model_validation_error(
                    &format!(
                        "The {}index definition refers to the unknown fields {}.",
                        if index_data.is_unique { "unique " } else { "" },
                        unresolvable_fields.join(", "),
                    ),
                    ctx.db.ast()[model_id].name(),
                    args.span(),
                ));
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
                        attribute_name = if index_data.is_unique { "unique" } else { "index" },
                        fields = suggested_fields.join(", ")
                    )
                } else {
                    String::new()
                };

                ctx.push_error(DatamodelError::new_model_validation_error(
                    &format!(
                        "The {prefix}index definition refers to the relation fields {the_fields}. Index definitions must reference only scalar fields.{suggestion}",
                        prefix = if index_data.is_unique { "unique " } else { "" },
                        the_fields = relation_fields.iter().map(|(f, _)| f.name()).join(", "),
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
    field_id: ast::FieldId,
    model_data: &ModelData<'ast>,
    relation_field: &mut RelationField<'ast>,
    ctx: &mut Context<'ast>,
) {
    if let Some(fields) = args.optional_arg("fields") {
        let fields = match resolve_field_array(&fields, args.span(), model_id, ctx) {
            Ok(fields) => fields,
            Err(FieldResolutionError::AlreadyDealtWith) => Vec::new(),
            Err(FieldResolutionError::ProblematicFields {
                unknown_fields: unresolvable_fields,
                relation_fields,
            }) => {
                if !unresolvable_fields.is_empty() {
                    ctx.push_error(DatamodelError::new_validation_error(&format!("The argument fields must refer only to existing fields. The following fields do not exist in this model: {}", unresolvable_fields.join(", ")), fields.span()))
                }

                if !relation_fields.is_empty() {
                    ctx.push_error(DatamodelError::new_validation_error(&format!("The argument fields must refer only to scalar fields. But it is referencing the following relation fields: {}", relation_fields.iter().map(|(f, _)| f.name()).join(", ")), fields.span()));
                }

                Vec::new()
            }
        };

        relation_field.fields = Some(fields);
    }

    relation::validate_relation_field_arity(model_id, field_id, relation_field, ctx);

    if let Some(references) = args.optional_arg("references") {
        let references = match resolve_field_array(&references, args.span(), relation_field.referenced_model, ctx) {
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
                    ctx.push_error(DatamodelError::new_validation_error(&msg, args.span()));
                }

                if !relation_fields.is_empty() {
                    let msg = format!(
                        "The argument `references` must refer only to scalar fields in the related model `{}`. But it is referencing the following relation fields: {}",
                        ctx.db.ast[relation_field.referenced_model].name(),
                        relation_fields.iter().map(|(f, _)| f.name()).join(", "),
                    );
                    ctx.push_error(DatamodelError::new_validation_error(&msg, args.span()));
                }

                Vec::new()
            }
        };

        relation_field.references = Some(references);
    }

    // Validate the `name` argument if present.
    match args.optional_default_arg("name").map(|arg| arg.as_str()) {
        Some(Ok("")) => ctx.push_error(args.new_attribute_validation_error("A relation cannot have an empty name.")),
        Some(Ok(name)) => {
            relation_field.name = Some(name);
        }
        Some(Err(err)) => ctx.push_error(err),
        None => (),
    }

    // Validate referential actions.
    if let Some(on_delete) = args.optional_arg("onDelete") {
        match on_delete.as_referential_action() {
            Ok(action) => {
                relation_field.on_delete = Some(action);
            }
            Err(err) => ctx.push_error(err),
        }
    }

    if let Some(on_update) = args.optional_arg("onUpdate") {
        match on_update.as_referential_action() {
            Ok(action) => {
                relation_field.on_update = Some(action);
            }
            Err(err) => ctx.push_error(err),
        }
    }

    let fk_name = {
        let ast_model = &ctx.db.ast[model_id];
        //TODO(matthias) when is relationfield.fields populated exactly ????
        let generated_name = if let Some(fields) = &relation_field.fields {
            let table_name = model_data.mapped_name.unwrap_or(&ast_model.name.name);
            let column_names = get_field_db_names(model_id, fields, ctx);

            Some(
                ConstraintNames::foreign_key_constraint_name(table_name, &column_names, ctx.db.active_connector())
                    .into(),
            )
        } else {
            None
        };
        //TODO(matthias) error when map given on the wrong side, thats why this is not dryed up
        let db_name = match args.optional_arg("map").map(|name| name.as_str()) {
            Some(Ok("")) => error_on_empty_string_cow(args, ctx),
            Some(Ok(name)) => {
                if !ctx.db.active_connector().supports_named_foreign_keys() {
                    ctx.push_error(
                        args.new_attribute_validation_error("Your provider does not support named foreign keys."),
                    )
                }
                Some(name.into())
            }
            Some(Err(err)) => push_error_cow(err, ctx),
            None => generated_name,
        };

        validate_db_name(ast_model, args, &db_name, "@relation", ctx);

        db_name
    };

    relation_field.fk_name = fk_name;
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
fn resolve_field_array<'ast>(
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

//helpers
pub(super) fn visit_map_attribute<'ast>(map_args: &mut Arguments<'ast>, ctx: &mut Context<'ast>) -> Option<&'ast str> {
    match map_args.default_arg("name").map(|value| value.as_str()) {
        Ok(Ok(name)) => return Some(name),
        Err(err) => ctx.push_error(err), // not flattened for error handing legacy reasons
        Ok(Err(err)) => ctx.push_error(map_args.new_attribute_validation_error(&err.to_string())),
    };

    None
}

fn get_map_argument<'ast>(
    args: &mut Arguments<'ast>,
    generated_name: String,
    ctx: &mut Context<'ast>,
) -> Option<Cow<'ast, str>> {
    match args.optional_arg("map").map(|name| name.as_str()) {
        Some(Ok("")) => error_on_empty_string_cow(args, ctx),
        Some(Ok(name)) => Some(name.into()),
        Some(Err(err)) => push_error_cow(err, ctx),
        None => Some(generated_name.into()),
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

fn get_field_db_names<'ast>(model_id: ModelId, fields: &[FieldId], ctx: &mut Context<'ast>) -> Vec<&'ast str> {
    fields
        .iter()
        .map(|&field_id| {
            ctx.db
                .get_field_database_name(model_id, field_id)
                .unwrap_or(&ctx.db.ast[model_id][field_id].name.name)
        })
        .collect()
}

// errors
fn push_error_cow<'ast>(err: DatamodelError, ctx: &mut Context<'ast>) -> Option<Cow<'ast, str>> {
    ctx.push_error(err);
    None
}

fn error_on_empty_string_cow<'ast>(args: &mut Arguments<'ast>, ctx: &mut Context<'ast>) -> Option<Cow<'ast, str>> {
    ctx.push_error(args.new_attribute_validation_error("The `map` argument cannot be an empty string."));
    None
}

fn validate_db_name<'ast>(
    ast_model: &'ast Model,
    args: &mut Arguments<'ast>,
    db_name: &Option<Cow<'ast, str>>,
    attribute: &'ast str,
    ctx: &mut Context<'ast>,
) {
    if let Some(err) = ConstraintNames::is_db_name_too_long(
        args.span(),
        ast_model.name(),
        db_name,
        attribute,
        ctx.db.active_connector(),
    ) {
        ctx.push_error(err);
    }
}

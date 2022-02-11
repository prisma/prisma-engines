use crate::{
    ast::{self, WithName},
    context::Context,
    types::{CompositeTypeField, DefaultAttribute, ScalarField, ScalarFieldType, ScalarType},
    DatamodelError, StringId,
};

/// @default on model scalar fields
pub(super) fn visit_model_field_default(
    field_data: &mut ScalarField,
    model_id: ast::ModelId,
    field_id: ast::FieldId,
    ctx: &mut Context<'_>,
) {
    let (argument_idx, value) = match ctx.visit_default_arg_with_idx("value") {
        Ok(value) => value,
        Err(err) => return ctx.push_error(err),
    };

    let ast_model = &ctx.ast[model_id];
    let ast_field = &ast_model[field_id];

    if ast_field.arity.is_list() {
        return ctx.push_attribute_validation_error("Cannot set a default value on list field.");
    }

    let mapped_name = default_attribute_mapped_name(ctx);
    let default_attribute_id = ctx.current_attribute_id();

    let mut accept = || {
        let default_value = DefaultAttribute {
            argument_idx,
            mapped_name,
            default_attribute: default_attribute_id,
        };

        field_data.default = Some(default_value);
    };

    // Resolve the default to a DefaultValue. We must loop in order to
    // resolve type aliases.
    let mut r#type = field_data.r#type;

    loop {
        match r#type {
            ScalarFieldType::CompositeType(ctid) => {
                validate_default_value_on_composite_type(ctid, ast_field, ctx);
            }
            ScalarFieldType::Enum(enum_id) => {
                match value.value {
                    ast::Expression::ConstantValue(enum_value, _) => {
                        if ctx.ast[enum_id].values.iter().any(|v| v.name() == enum_value) {
                            accept()
                        } else {
                            validate_invalid_default_enum_value(enum_value, ctx);
                        }
                    }
                    ast::Expression::Function(funcname, funcargs, _) if funcname == FN_DBGENERATED => {
                        validate_dbgenerated_args(&funcargs.arguments, accept, ctx);
                    }
                    bad_value => validate_invalid_default_enum_expr(bad_value, ctx),
                };
            }
            ScalarFieldType::BuiltInScalar(scalar_type) => {
                validate_model_builtin_scalar_type_default(scalar_type, value.value, mapped_name, accept, ctx)
            }
            ScalarFieldType::Alias(alias_id) => {
                r#type = ctx.types.type_aliases[&alias_id];
                continue;
            }
            ScalarFieldType::Unsupported(_) => match value.value {
                ast::Expression::Function(funcname, funcargs, _) if funcname == FN_DBGENERATED => {
                    validate_dbgenerated_args(&funcargs.arguments, accept, ctx);
                }
                _ => ctx
                    .push_attribute_validation_error("Only @default(dbgenerated()) can be used for Unsupported types."),
            },
        }

        break;
    }
}

/// @default on composite type fields
pub(super) fn visit_composite_field_default(
    field_data: &mut CompositeTypeField,
    ct_id: ast::CompositeTypeId,
    field_id: ast::FieldId,
    ctx: &mut Context<'_>,
) {
    let (argument_idx, value) = match ctx.visit_default_arg_with_idx("value") {
        Ok(value) => value,
        Err(err) => return ctx.push_error(err),
    };

    let ast_model = &ctx.ast[ct_id];
    let ast_field = &ast_model[field_id];

    if ast_field.arity.is_list() {
        return ctx.push_attribute_validation_error("Cannot set a default value on list field.");
    }

    if ctx.visit_optional_arg("map").is_some() {
        ctx.push_attribute_validation_error("The `map` argument is not allowed on a composite type field.");
    }

    let default_attribute = ctx.current_attribute_id();

    let mut accept = || {
        let default_value = DefaultAttribute {
            argument_idx,
            mapped_name: None,
            default_attribute,
        };

        field_data.default = Some(default_value);
    };

    // Resolve the default to a DefaultValue. We must loop in order to
    // resolve type aliases.
    match field_data.r#type {
        ScalarFieldType::CompositeType(ctid) => {
            validate_default_value_on_composite_type(ctid, ast_field, ctx);
        }
        ScalarFieldType::Enum(enum_id) => {
            match value.value {
                ast::Expression::ConstantValue(enum_value, _) => {
                    if ctx.ast[enum_id].values.iter().any(|v| v.name() == enum_value) {
                        accept()
                    } else {
                        validate_invalid_default_enum_value(enum_value, ctx);
                    }
                }
                bad_value => validate_invalid_default_enum_expr(bad_value, ctx),
            };
        }
        ScalarFieldType::BuiltInScalar(scalar_type) => {
            validate_composite_builtin_scalar_type_default(scalar_type, value.value, accept, ctx)
        }
        ScalarFieldType::Unsupported(_) => {
            ctx.push_attribute_validation_error("Composite field of type `Unsupported` cannot have default values.")
        }
        ScalarFieldType::Alias(_) => unreachable!(),
    }
}

fn validate_model_builtin_scalar_type_default(
    scalar_type: ScalarType,
    value: &ast::Expression,
    mapped_name: Option<StringId>,
    mut accept: impl FnMut(),
    ctx: &mut Context<'_>,
) {
    match (scalar_type, value) {
        (ScalarType::String, ast::Expression::StringValue(_, _))
        | (ScalarType::Json, ast::Expression::StringValue(_, _))
        | (ScalarType::Bytes, ast::Expression::StringValue(_, _))
        | (ScalarType::Int, ast::Expression::NumericValue(_, _))
        | (ScalarType::BigInt, ast::Expression::NumericValue(_, _))
        | (ScalarType::Float, ast::Expression::NumericValue(_, _))
        | (ScalarType::DateTime, ast::Expression::StringValue(_, _))
        | (ScalarType::Decimal, ast::Expression::NumericValue(_, _))
        | (ScalarType::Decimal, ast::Expression::StringValue(_, _)) => accept(),
        (ScalarType::Boolean, ast::Expression::ConstantValue(val, span)) => {
            validate_default_bool_value(val, *span, accept, ctx)
        }

        // Functions
        (_, ast::Expression::Function(funcname, _, _)) if funcname == FN_AUTOINCREMENT && mapped_name.is_some() => {
            ctx.push_attribute_validation_error("Naming an autoincrement default value is not allowed.")
        }
        (ScalarType::Int, ast::Expression::Function(funcname, funcargs, _))
        | (ScalarType::BigInt, ast::Expression::Function(funcname, funcargs, _))
            if funcname == FN_AUTOINCREMENT =>
        {
            validate_empty_function_args(funcname, &funcargs.arguments, accept, ctx)
        }
        (ScalarType::String, ast::Expression::Function(funcname, funcargs, _))
            if funcname == FN_UUID || funcname == FN_CUID =>
        {
            validate_empty_function_args(funcname, &funcargs.arguments, accept, ctx)
        }
        (ScalarType::DateTime, ast::Expression::Function(funcname, funcargs, _)) if funcname == FN_NOW => {
            validate_empty_function_args(FN_NOW, &funcargs.arguments, accept, ctx)
        }

        (_, ast::Expression::Function(funcname, funcargs, _)) if funcname == FN_AUTO => {
            validate_auto_args(&funcargs.arguments, accept, ctx)
        }

        (_, ast::Expression::Function(funcname, funcargs, _)) if funcname == FN_DBGENERATED => {
            validate_dbgenerated_args(&funcargs.arguments, accept, ctx)
        }

        (_, ast::Expression::Function(funcname, _, _)) if !KNOWN_FUNCTIONS.contains(&funcname.as_str()) => {
            validate_unknown_function_default(funcname, ctx);
        }

        // Invalid function default.
        (scalar_type, ast::Expression::Function(funcname, _, _)) => {
            validate_invalid_funtion_default(funcname, scalar_type, ctx);
        }

        // Invalid scalar default.
        (scalar_type, value) => {
            validate_invalid_scalar_default(scalar_type, value, ctx);
        }
    }
}

fn validate_composite_builtin_scalar_type_default(
    scalar_type: ScalarType,
    value: &ast::Expression,
    mut accept: impl FnMut(),
    ctx: &mut Context<'_>,
) {
    match (scalar_type, value) {
        (ScalarType::String, ast::Expression::StringValue(_, _))
        | (ScalarType::Json, ast::Expression::StringValue(_, _))
        | (ScalarType::Bytes, ast::Expression::StringValue(_, _))
        | (ScalarType::Int, ast::Expression::NumericValue(_, _))
        | (ScalarType::BigInt, ast::Expression::NumericValue(_, _))
        | (ScalarType::Float, ast::Expression::NumericValue(_, _))
        | (ScalarType::DateTime, ast::Expression::StringValue(_, _))
        | (ScalarType::Decimal, ast::Expression::NumericValue(_, _))
        | (ScalarType::Decimal, ast::Expression::StringValue(_, _)) => accept(),

        (ScalarType::Boolean, ast::Expression::ConstantValue(val, span)) => {
            validate_default_bool_value(val, *span, accept, ctx)
        }
        // Functions
        (ScalarType::String, ast::Expression::Function(funcname, funcargs, _))
            if funcname == FN_UUID || funcname == FN_CUID =>
        {
            validate_empty_function_args(funcname, &funcargs.arguments, accept, ctx)
        }
        (ScalarType::DateTime, ast::Expression::Function(funcname, funcargs, _)) if funcname == FN_NOW => {
            validate_empty_function_args(FN_NOW, &funcargs.arguments, accept, ctx)
        }
        (_, ast::Expression::Function(funcname, _, _))
            if funcname == FN_DBGENERATED || funcname == FN_AUTOINCREMENT || funcname == FN_AUTO =>
        {
            ctx.push_attribute_validation_error(&format!(
                "The function `{funcname}()` is not supported on composite fields.",
            ));
        }
        (_, ast::Expression::Function(funcname, _, _)) if !KNOWN_FUNCTIONS.contains(&funcname.as_str()) => {
            validate_unknown_function_default(funcname, ctx);
        }
        // Invalid function default.
        (scalar_type, ast::Expression::Function(funcname, _, _)) => {
            validate_invalid_funtion_default(funcname, scalar_type, ctx);
        }
        // Invalid scalar default.
        (scalar_type, value) => {
            validate_invalid_scalar_default(scalar_type, value, ctx);
        }
    }
}

fn default_attribute_mapped_name(ctx: &mut Context<'_>) -> Option<StringId> {
    match ctx.visit_optional_arg("map").map(|name| name.as_str()) {
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
    }
}

fn validate_default_bool_value(
    bool_value: &str,
    span: diagnostics::Span,
    mut accept: impl FnMut(),
    ctx: &mut Context<'_>,
) {
    match bool_value {
        "true" | "false" => accept(),
        _ => ctx.push_error(DatamodelError::new_attribute_validation_error(
            "A boolean literal must be `true` or `false`.",
            "default",
            span,
        )),
    }
}

fn validate_invalid_default_enum_value(enum_value: &str, ctx: &mut Context<'_>) {
    ctx.push_attribute_validation_error(&format!(
        "The defined default value `{enum_value}` is not a valid value of the enum specified for the field."
    ));
}

fn validate_invalid_default_enum_expr(bad_value: &ast::Expression, ctx: &mut Context<'_>) {
    ctx.push_attribute_validation_error(&format!("Expected an enum value, but found `{bad_value}`."))
}

fn validate_unknown_function_default(fn_name: &str, ctx: &mut Context<'_>) {
    ctx.push_attribute_validation_error(&format!(
        "The function `{fn_name}` is not a known function. You can read about the available functions here: https://pris.ly/d/attribute-functions.",
    ));
}

fn validate_invalid_scalar_default(scalar_type: ScalarType, value: &ast::Expression, ctx: &mut Context<'_>) {
    ctx.push_attribute_validation_error(&format!(
        "Expected a {scalar_type} value, but found `{bad_value}`.",
        scalar_type = scalar_type.as_str(),
        bad_value = value
    ));
}

fn validate_invalid_funtion_default(fn_name: &str, scalar_type: ScalarType, ctx: &mut Context<'_>) {
    ctx.push_attribute_validation_error(&format!(
        "The function `{fn_name}()` cannot be used on fields of type `{scalar_type}`.",
        scalar_type = scalar_type.as_str()
    ));
}

fn validate_default_value_on_composite_type(ctid: ast::CompositeTypeId, ast_field: &ast::Field, ctx: &mut Context<'_>) {
    let attr = ctx.current_attribute();
    let ct_name = &ctx.ast[ctid].name.name;

    ctx.push_error(DatamodelError::new_composite_type_field_validation_error(
        "Defaults on fields of type composite are not supported. Please remove the `@default` attribute.",
        ct_name,
        &ast_field.name.name,
        attr.span,
    ));
}

fn validate_empty_function_args(
    fn_name: &str,
    args: &[ast::Argument],
    mut accept: impl FnMut(),
    ctx: &mut Context<'_>,
) {
    if args.is_empty() {
        return accept();
    }

    ctx.push_attribute_validation_error(&format!(
        "The `{fn_name}` function does not take any argument. Consider changing this default to `{fn_name}()`.",
        fn_name = fn_name
    ));
}

fn validate_auto_args(args: &[ast::Argument], mut accept: impl FnMut(), ctx: &mut Context<'_>) {
    if !args.is_empty() {
        ctx.push_attribute_validation_error("`auto()` takes no arguments");
    } else {
        accept()
    }
}

fn validate_dbgenerated_args(args: &[ast::Argument], mut accept: impl FnMut(), ctx: &mut Context<'_>) {
    let mut bail = || {
        // let's not mention what we don't want to see.
        ctx.push_attribute_validation_error("`dbgenerated()` takes a single String argument")
    };

    if args.len() > 1 {
        bail()
    }

    match args.get(0).map(|arg| &arg.value) {
        Some(ast::Expression::StringValue(val, _)) if val.is_empty() => {
            ctx.push_attribute_validation_error(
                "dbgenerated() takes either no argument, or a single nonempty string argument.",
            );
        }
        None | Some(ast::Expression::StringValue(_, _)) => accept(),
        _ => bail(),
    }
}

const FN_AUTOINCREMENT: &str = "autoincrement";
const FN_CUID: &str = "cuid";
const FN_DBGENERATED: &str = "dbgenerated";
const FN_NOW: &str = "now";
const FN_UUID: &str = "uuid";
const FN_AUTO: &str = "auto";

const KNOWN_FUNCTIONS: &[&str] = &[FN_AUTOINCREMENT, FN_CUID, FN_DBGENERATED, FN_NOW, FN_UUID, FN_AUTO];

use crate::{
    ast::{self, WithName},
    context::{Arguments, Context},
    types::{DefaultAttribute, ScalarField, ScalarFieldType, ScalarType},
    DatamodelError,
};

/// @default on scalar fields
pub(super) fn visit_field_default<'ast>(
    args: &mut Arguments<'ast>,
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

    let mapped_name = default_attribute_mapped_name(args, ctx);
    let default_attribute = args.attribute();

    let mut accept = || {
        let default_value = DefaultAttribute {
            value: value.value,
            mapped_name,
            default_attribute,
        };

        field_data.default = Some(default_value);
    };

    // Resolve the default to a DefaultValue. We must loop in order to
    // resolve type aliases.
    let mut r#type = field_data.r#type;

    loop {
        match r#type {
            ScalarFieldType::CompositeType(ctid) => {
                let ct_name = ctx.db.walk_composite_type(ctid).name();
                ctx.push_error(DatamodelError::new_composite_type_field_validation_error(
                    "Defaults inside composite types are not supported",
                    ct_name,
                    &ast_field.name.name,
                    args.span(),
                ));
            }
            ScalarFieldType::Enum(enum_id) => {
                match value.value {
                    ast::Expression::ConstantValue(enum_value, _) => {
                        if ctx.db.ast[enum_id].values.iter().any(|v| v.name() == enum_value) {
                            accept()
                        } else {
                            ctx.push_error(args.new_attribute_validation_error(
                                "The defined default value is not a valid value of the enum specified for the field.",
                            ))
                        }
                    }
                    ast::Expression::Function(funcname, funcargs, _) if funcname == FN_DBGENERATED => {
                        validate_dbgenerated_args(funcargs, args, accept, ctx);
                    }
                    value => ctx.push_error(args.new_attribute_validation_error(&format!(
                        "Expected a an enum value, but found `{bad_value}`.",
                        bad_value = value
                    ))),
                };
            }
            ScalarFieldType::BuiltInScalar(scalar_type) => {
                validate_builtin_scalar_type_default(scalar_type, value.value, mapped_name, accept, args, ctx)
            }
            ScalarFieldType::Alias(alias_id) => {
                r#type = ctx.db.types.type_aliases[&alias_id];
                continue;
            }
            ScalarFieldType::Unsupported => {
                match value.value {
                    ast::Expression::Function(funcname, funcargs, _) if funcname == FN_DBGENERATED => {
                        validate_dbgenerated_args(&funcargs, &args, accept, ctx);
                    }
                    _ => ctx.push_error(args.new_attribute_validation_error(
                        "Only @default(dbgenerated()) can be used for Unsupported types.",
                    )),
                }
            }
        }

        break;
    }
}

fn validate_builtin_scalar_type_default(
    scalar_type: ScalarType,
    value: &ast::Expression,
    mapped_name: Option<&str>,
    mut accept: impl FnMut(),
    args: &Arguments<'_>,
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

        (ScalarType::Boolean, ast::Expression::ConstantValue(val, span)) => match val.as_str() {
            "true" | "false" => accept(),
            _ => ctx.push_error(DatamodelError::new_attribute_validation_error(
                "A boolean literal must be `true` or `false`.",
                "default",
                *span,
            )),
        },

        // Functions
        (_, ast::Expression::Function(funcname, _, _)) if funcname == FN_AUTOINCREMENT && mapped_name.is_some() => {
            ctx.push_error(args.new_attribute_validation_error("Naming an autoincrement default value is not allowed."))
        }
        (ScalarType::Int, ast::Expression::Function(funcname, funcargs, _))
        | (ScalarType::BigInt, ast::Expression::Function(funcname, funcargs, _))
            if funcname == FN_AUTOINCREMENT =>
        {
            validate_empty_function_args(funcname, funcargs, args, accept, ctx)
        }
        (ScalarType::String, ast::Expression::Function(funcname, funcargs, _))
            if funcname == FN_UUID || funcname == FN_CUID =>
        {
            validate_empty_function_args(funcname, funcargs, args, accept, ctx)
        }
        (ScalarType::DateTime, ast::Expression::Function(funcname, funcargs, _)) if funcname == FN_NOW => {
            validate_empty_function_args(FN_NOW, funcargs, args, accept, ctx)
        }

        (_, ast::Expression::Function(funcname, funcargs, _)) if funcname == FN_DBGENERATED => {
            validate_dbgenerated_args(funcargs, args, accept, ctx)
        }

        (_, ast::Expression::Function(funcname, _, _)) if !KNOWN_FUNCTIONS.contains(&funcname.as_str()) => {
            ctx.push_error(args.new_attribute_validation_error(&format!(
                            "The function `{funcname}` is not a known function. You can read about the available functions here: https://pris.ly/d/attribute-functions",
                            funcname = funcname
                        )));
        }

        // Invalid function default.
        (scalar_type, ast::Expression::Function(funcname, _, _)) => {
            ctx.push_error(args.new_attribute_validation_error(&format!(
                "The function `{funcname}()` cannot be used on fields of type `{scalar_type}`.",
                funcname = funcname,
                scalar_type = scalar_type.as_str()
            )));
        }

        // Invalid scalar default.
        (scalar_type, value) => ctx.push_error(args.new_attribute_validation_error(&format!(
            "Expected a {scalar_type} value, but found `{bad_value}`.",
            scalar_type = scalar_type.as_str(),
            bad_value = value
        ))),
    }
}

fn default_attribute_mapped_name<'ast>(args: &mut Arguments<'ast>, ctx: &mut Context<'ast>) -> Option<&'ast str> {
    match args.optional_arg("map").map(|name| name.as_str()) {
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
    }
}

fn validate_empty_function_args(
    fn_name: &str,
    args: &[ast::Argument],
    arguments: &Arguments<'_>,
    mut accept: impl FnMut(),
    ctx: &mut Context<'_>,
) {
    if args.is_empty() {
        return accept();
    }

    ctx.push_error(arguments.new_attribute_validation_error(&format!(
        "The `{fn_name}` function does not take any argument. Consider changing this default to `{fn_name}()`.",
        fn_name = fn_name
    )));
}

fn validate_dbgenerated_args(
    args: &[ast::Argument],
    arguments: &Arguments<'_>,
    mut accept: impl FnMut(),
    ctx: &mut Context<'_>,
) {
    match args.len() {
        0 => accept(),
        1 => {
            let arg = &args[0];

            if !arg.name.name.is_empty() {
                ctx.push_error(
                    arguments.new_attribute_validation_error("dbgenerated() does not take a named argument."),
                )
            }

            match &arg.value {
                ast::Expression::StringValue(val, _) if !val.is_empty() => accept(),
                _ => {
                    ctx.push_error(arguments.new_attribute_validation_error(
                        "dbgenerated() takes either no argument, or a single nonempty string argument.",
                    ));
                }
            }
        }
        _ => ctx.push_error(arguments.new_attribute_validation_error("`dbgenerated()` takes a single String argument")), // let's not mention what we don't want to see.
    };
}

const FN_AUTOINCREMENT: &str = "autoincrement";
const FN_CUID: &str = "cuid";
const FN_DBGENERATED: &str = "dbgenerated";
const FN_NOW: &str = "now";
const FN_UUID: &str = "uuid";

const KNOWN_FUNCTIONS: &[&str] = &[FN_AUTOINCREMENT, FN_CUID, FN_DBGENERATED, FN_NOW, FN_UUID];

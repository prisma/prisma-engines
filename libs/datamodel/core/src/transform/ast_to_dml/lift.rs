use super::super::directives::AllDirectives;
use crate::preview_features::PreviewFeatures;
use crate::transform::helpers::ValueValidator;
use crate::{
    ast, configuration, dml,
    error::{DatamodelError, ErrorCollection},
    Field, FieldType, ScalarType,
};
use datamodel_connector::error::{ConnectorError, ErrorKind};
use itertools::Itertools;

/// Helper for lifting a datamodel.
///
/// When lifting, the
/// AST is converted to the real datamodel, and
/// additional semantics are attached.
pub struct LiftAstToDml<'a> {
    directives: AllDirectives,
    source: Option<&'a configuration::Datasource>,
}

impl<'a> LiftAstToDml<'a> {
    /// Creates a new instance, with all builtin directives and
    /// the directives defined by the given sources registered.
    ///
    /// The directives defined by the given sources will be namespaced.
    pub fn new(source: Option<&'a configuration::Datasource>) -> LiftAstToDml {
        LiftAstToDml {
            directives: AllDirectives::new(),
            source,
        }
    }

    pub fn lift(&self, ast_schema: &ast::SchemaAst) -> Result<dml::Datamodel, ErrorCollection> {
        let mut schema = dml::Datamodel::new();
        let mut errors = ErrorCollection::new();

        for ast_obj in &ast_schema.tops {
            match ast_obj {
                ast::Top::Enum(en) => match self.lift_enum(&en) {
                    Ok(en) => schema.add_enum(en),
                    Err(mut err) => errors.append(&mut err),
                },
                ast::Top::Model(ty) => match self.lift_model(&ty, ast_schema) {
                    Ok(md) => schema.add_model(md),
                    Err(mut err) => errors.append(&mut err),
                },
                ast::Top::Source(_) => { /* Source blocks are explicitly ignored by the validator */ }
                ast::Top::Generator(_) => { /* Generator blocks are explicitly ignored by the validator */ }
                // TODO: For now, type blocks are never checked on their own.
                ast::Top::Type(_) => { /* Type blocks are inlined */ }
            }
        }

        if errors.has_errors() {
            Err(errors)
        } else {
            Ok(schema)
        }
    }

    /// Internal: Validates a model AST node and lifts it to a DML model.
    fn lift_model(&self, ast_model: &ast::Model, ast_schema: &ast::SchemaAst) -> Result<dml::Model, ErrorCollection> {
        let mut model = dml::Model::new(ast_model.name.name.clone(), None);
        model.documentation = ast_model.documentation.clone().map(|comment| comment.text);

        let mut errors = ErrorCollection::new();

        for ast_field in &ast_model.fields {
            match self.lift_field(ast_field, ast_schema) {
                Ok(field) => model.add_field(field),
                Err(mut err) => errors.append(&mut err),
            }
        }

        if let Err(mut err) = self.directives.model.validate_and_apply(ast_model, &mut model) {
            errors.append(&mut err);
        }

        if errors.has_errors() {
            return Err(errors);
        }

        Ok(model)
    }

    /// Internal: Validates an enum AST node.
    fn lift_enum(&self, ast_enum: &ast::Enum) -> Result<dml::Enum, ErrorCollection> {
        let mut errors = ErrorCollection::new();

        let supports_enums = match self.source {
            Some(source) => source.combined_connector.supports_enums(),
            None => true,
        };
        if !supports_enums {
            errors.push(DatamodelError::new_validation_error(
                &format!(
                    "You defined the enum `{}`. But the current connector does not support enums.",
                    &ast_enum.name.name
                ),
                ast_enum.span,
            ));
            return Err(errors);
        }

        let mut en = dml::Enum::new(&ast_enum.name.name, vec![]);

        for ast_enum_value in &ast_enum.values {
            match self.lift_enum_value(ast_enum_value) {
                Ok(value) => en.add_value(value),
                Err(mut err) => errors.append(&mut err),
            }
        }

        if en.values.len() == 0 {
            errors.push(DatamodelError::new_validation_error(
                "An enum must have at least one value.",
                ast_enum.span,
            ))
        }

        en.documentation = ast_enum.documentation.clone().map(|comment| comment.text);

        if let Err(mut err) = self.directives.enm.validate_and_apply(ast_enum, &mut en) {
            errors.append(&mut err);
        }

        if errors.has_errors() {
            Err(errors)
        } else {
            Ok(en)
        }
    }

    /// Internal: Validates an enum value AST node.
    fn lift_enum_value(&self, ast_enum_value: &ast::EnumValue) -> Result<dml::EnumValue, ErrorCollection> {
        let mut enum_value = dml::EnumValue::new(&ast_enum_value.name.name);
        enum_value.documentation = ast_enum_value.documentation.clone().map(|comment| comment.text);

        self.directives
            .enm_value
            .validate_and_apply(ast_enum_value, &mut enum_value)?;

        Ok(enum_value)
    }

    /// Internal: Lift a field AST node to a DML field.
    fn lift_field(&self, ast_field: &ast::Field, ast_schema: &ast::SchemaAst) -> Result<dml::Field, ErrorCollection> {
        let mut errors = ErrorCollection::new();
        // If we cannot parse the field type, we exit right away.
        let (field_type, extra_attributes) = self.lift_field_type(&ast_field, None, ast_schema, &mut Vec::new())?;

        let mut field = match field_type {
            FieldType::Relation(info) => {
                let arity = self.lift_field_arity(&ast_field.arity);
                let mut field = dml::RelationField::new(&ast_field.name.name, arity, info);
                field.documentation = ast_field.documentation.clone().map(|comment| comment.text);
                Field::RelationField(field)
            }
            x => {
                let arity = self.lift_field_arity(&ast_field.arity);
                let mut field = dml::ScalarField::new(&ast_field.name.name, arity, x);
                field.documentation = ast_field.documentation.clone().map(|comment| comment.text);
                Field::ScalarField(field)
            }
        };

        // We merge attributes so we can fail on duplicates.
        let attributes = [&extra_attributes[..], &ast_field.directives[..]].concat();

        if let Err(mut err) = self.directives.field.validate_and_apply(&attributes, &mut field) {
            errors.append(&mut err);
        }

        if errors.has_errors() {
            Err(errors)
        } else {
            Ok(field)
        }
    }

    /// Internal: Lift a field's arity.
    fn lift_field_arity(&self, ast_field: &ast::FieldArity) -> dml::FieldArity {
        match ast_field {
            ast::FieldArity::Required => dml::FieldArity::Required,
            ast::FieldArity::Optional => dml::FieldArity::Optional,
            ast::FieldArity::List => dml::FieldArity::List,
        }
    }

    /// Internal: Lift a field's type.
    /// Auto resolves custom types and gathers directives, but without a stack overflow please.
    fn lift_field_type(
        &self,
        ast_field: &ast::Field,
        type_alias: Option<String>,
        ast_schema: &ast::SchemaAst,
        checked_types: &mut Vec<String>,
    ) -> Result<(dml::FieldType, Vec<ast::Directive>), DatamodelError> {
        let type_name = &ast_field.field_type.name;

        let (supports_native_types, datasource_name) = match self.source {
            Some(source) => (source.has_preview_feature("nativeTypes"), source.name.as_str()),
            _ => (false, ""),
        };

        if let Ok(scalar_type) = ScalarType::from_str(type_name) {
            if supports_native_types {
                let (connector_string, connector) = (
                    &self.source.unwrap().active_provider,
                    &self.source.unwrap().active_connector,
                );

                let prefix = format!("{}{}", datasource_name, ".");

                let type_specifications = ast_field
                    .directives
                    .iter()
                    .filter(|dir| dir.name.name.starts_with(&prefix))
                    .collect_vec();

                let type_specifications_with_invalid_datasource_name = ast_field
                    .directives
                    .iter()
                    .filter(|dir| dir.name.name.contains(".") && !dir.name.name.starts_with(&prefix))
                    .collect_vec();

                if type_specifications_with_invalid_datasource_name.len() > 0 {
                    let incorrect_type_specification =
                        type_specifications_with_invalid_datasource_name.first().unwrap();
                    let mut type_specification_name_split = incorrect_type_specification.name.name.split(".");
                    let given_prefix = type_specification_name_split.next().unwrap();
                    return Err(DatamodelError::new_connector_error(
                        &ConnectorError::from_kind(ErrorKind::InvalidPrefixForNativeTypes {
                            given_prefix: String::from(given_prefix),
                            expected_prefix: String::from(datasource_name),
                            suggestion: format!("{}{}", prefix, type_specification_name_split.next().unwrap()),
                        })
                        .to_string(),
                        incorrect_type_specification.span,
                    ));
                }

                let type_specification = type_specifications.first();

                if type_specifications.len() > 1 {
                    return Err(DatamodelError::new_duplicate_directive_error(
                        &prefix,
                        type_specification.unwrap().span,
                    ));
                }

                let name = type_specification.map(|dir| dir.name.name.trim_start_matches(&prefix));

                // convert arguments to u32 if possible
                let number_args = type_specification.map(|dir| dir.arguments.clone());
                let args = if let Some(number) = number_args {
                    let p = number
                        .iter()
                        .map(|arg| ValueValidator::new(&arg.value).as_int())
                        .collect_vec();
                    if let Some(error) = p.iter().find(|arg| arg.is_err()) {
                        return Err(error.clone().err().unwrap());
                    }
                    p.iter().map(|arg| *arg.as_ref().unwrap() as u32).collect_vec()
                } else {
                    vec![]
                };

                if let Some(x) = name {
                    let constructor = if let Some(cons) = connector.find_native_type_constructor(x) {
                        cons
                    } else {
                        return Err(DatamodelError::new_connector_error(
                            &ConnectorError::from_kind(ErrorKind::NativeTypeNameUnknown {
                                native_type: x.parse().unwrap(),
                                connector_name: connector_string.clone(),
                            })
                            .to_string(),
                            type_specification.unwrap().span,
                        ));
                    };

                    let number_of_args = args.iter().count();
                    if number_of_args < constructor._number_of_args
                        || number_of_args > constructor._number_of_args + constructor._number_of_optional_args
                    {
                        return Err(DatamodelError::new_argument_count_missmatch_error(
                            x,
                            constructor._number_of_args,
                            number_of_args,
                            type_specification.unwrap().span,
                        ));
                    }

                    // check for compatability with scalar type
                    let compatable_prisma_scalar_type = constructor.prisma_type;
                    if compatable_prisma_scalar_type != scalar_type {
                        return Err(DatamodelError::new_connector_error(
                            &ConnectorError::from_kind(ErrorKind::IncompatibleNativeType {
                                native_type: x.parse().unwrap(),
                                field_type: scalar_type.to_string(),
                                expected_type: compatable_prisma_scalar_type.to_string(),
                            })
                            .to_string(),
                            type_specification.unwrap().span,
                        ));
                    }

                    let parse_native_type_result = connector.parse_native_type(x, args);
                    match parse_native_type_result {
                        Err(connector_error) => {
                            return Err(DatamodelError::new_connector_error(
                                &connector_error.to_string(),
                                type_specification.unwrap().span,
                            ))
                        }
                        Ok(parsed_native_type) => {
                            Ok((dml::FieldType::NativeType(scalar_type, parsed_native_type), vec![]))
                        }
                    }
                } else {
                    Ok((dml::FieldType::Base(scalar_type, type_alias), vec![]))
                }
            } else {
                if let Some(native_type_attribute) = ast_field.directives.iter().find(|d| d.name.name.contains(".")) {
                    return Err(DatamodelError::new_connector_error(
                        &ConnectorError::from_kind(ErrorKind::NativeFlagsPreviewFeatureDisabled).to_string(),
                        native_type_attribute.span,
                    ));
                } else {
                    Ok((dml::FieldType::Base(scalar_type, type_alias), vec![]))
                }
            }
        } else if ast_schema.find_model(type_name).is_some() {
            Ok((dml::FieldType::Relation(dml::RelationInfo::new(type_name)), vec![]))
        } else if ast_schema.find_enum(type_name).is_some() {
            Ok((dml::FieldType::Enum(type_name.clone()), vec![]))
        } else {
            self.resolve_custom_type(ast_field, ast_schema, checked_types)
        }
    }

    fn resolve_custom_type(
        &self,
        ast_field: &ast::Field,
        ast_schema: &ast::SchemaAst,
        checked_types: &mut Vec<String>,
    ) -> Result<(dml::FieldType, Vec<ast::Directive>), DatamodelError> {
        let type_name = &ast_field.field_type.name;

        if checked_types.iter().any(|x| x == type_name) {
            // Recursive type.
            return Err(DatamodelError::new_validation_error(
                &format!(
                    "Recursive type definitions are not allowed. Recursive path was: {} -> {}.",
                    checked_types.join(" -> "),
                    type_name
                ),
                ast_field.field_type.span,
            ));
        }

        if let Some(custom_type) = ast_schema.find_type_alias(&type_name) {
            checked_types.push(custom_type.name.name.clone());
            let (field_type, mut attrs) =
                self.lift_field_type(custom_type, Some(type_name.to_owned()), ast_schema, checked_types)?;

            if let dml::FieldType::Relation(_) = field_type {
                return Err(DatamodelError::new_validation_error(
                    "Only scalar types can be used for defining custom types.",
                    custom_type.field_type.span,
                ));
            }

            attrs.append(&mut custom_type.directives.clone());
            Ok((field_type, attrs))
        } else {
            Err(DatamodelError::new_type_not_found_error(
                type_name,
                ast_field.field_type.span,
            ))
        }
    }
}

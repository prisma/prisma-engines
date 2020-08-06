use super::super::directives::AllDirectives;
use crate::transform::helpers::ValueValidator;
use crate::{
    ast,
    common::ScalarType,
    configuration, dml,
    error::{DatamodelError, ErrorCollection},
    Field, FieldType,
};
use datamodel_connector::Connector;
use sql_datamodel_connector::SqlDatamodelConnectors;

/// Helper for lifting a datamodel.
///
/// When lifting, the
/// AST is converted to the real datamodel, and
/// additional semantics are attached.
pub struct LiftAstToDml<'a> {
    directives: AllDirectives,
    source: Option<&'a configuration::Datasource>,
}

// TODO carmen: feature flags of the Datasource must be used instead
const USE_CONNECTORS_FOR_CUSTOM_TYPES: bool = false; // FEATURE FLAG

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

        if let Ok(scalar_type) = ScalarType::from_str(type_name) {
            if USE_CONNECTORS_FOR_CUSTOM_TYPES {
                let pg_connector = SqlDatamodelConnectors::postgres();
                let pg_type_specification = ast_field.directives.iter().find(|dir| dir.name.name.starts_with("pg.")); // we use find because there should be at max 1.
                let name = pg_type_specification.map(|dir| dir.name.name.trim_start_matches("pg."));
                let args = pg_type_specification
                    .map(|dir| {
                        let args = dir
                            .arguments
                            .iter()
                            .map(|arg| ValueValidator::new(&arg.value).as_int().unwrap() as u32)
                            .collect();
                        args
                    })
                    .unwrap_or(vec![]);

                if let Some(x) = name.and_then(|ts| pg_connector.parse_native_type(&ts, args)) {
                    let field_type = dml::FieldType::NativeType(scalar_type, x);
                    Ok((field_type, vec![]))
                } else {
                    Ok((dml::FieldType::Base(scalar_type, type_alias), vec![]))
                }
            } else {
                Ok((dml::FieldType::Base(scalar_type, type_alias), vec![]))
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

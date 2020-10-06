#![allow(clippy::ptr_arg)] // some of the helpers take closures with references to strings

use anyhow::format_err;
use datamodel::ast::{self, ArgumentContainer, Identifier, SchemaAst};
use migration_connector::steps::{self, CreateSource, DeleteSource, MigrationStep};
use thiserror::Error;

pub trait DataModelCalculator: Send + Sync + 'static {
    fn infer(&self, current: &SchemaAst, steps: &[MigrationStep]) -> Result<SchemaAst, CalculatorError>;
}

#[derive(Debug, Error)]
#[error("{0}")]
pub struct CalculatorError(#[source] anyhow::Error);

impl From<anyhow::Error> for CalculatorError {
    fn from(fe: anyhow::Error) -> Self {
        CalculatorError(fe)
    }
}

pub struct DataModelCalculatorImpl;

impl DataModelCalculator for DataModelCalculatorImpl {
    fn infer(&self, current: &SchemaAst, steps: &[MigrationStep]) -> Result<SchemaAst, CalculatorError> {
        let cloned: SchemaAst = current.clone();
        apply(cloned, steps)
    }
}

fn apply(mut schema: SchemaAst, steps: &[MigrationStep]) -> Result<SchemaAst, CalculatorError> {
    for step in steps {
        apply_step(&mut schema, step)?;
    }

    Ok(schema)
}

fn apply_step(datamodel: &mut ast::SchemaAst, step: &MigrationStep) -> Result<(), CalculatorError> {
    match step {
        MigrationStep::CreateEnum(create_enum) => apply_create_enum(datamodel, create_enum)?,
        MigrationStep::UpdateEnum(update_enum) => apply_update_enum(datamodel, update_enum)?,
        MigrationStep::DeleteEnum(delete_enum) => apply_delete_enum(datamodel, delete_enum)?,
        MigrationStep::CreateModel(create_model) => apply_create_model(datamodel, create_model)?,
        MigrationStep::UpdateModel(update_model) => apply_update_model(datamodel, update_model)?,
        MigrationStep::DeleteModel(delete_model) => apply_delete_model(datamodel, delete_model)?,
        MigrationStep::CreateField(create_field) => apply_create_field(datamodel, create_field)?,
        MigrationStep::UpdateField(update_field) => apply_update_field(datamodel, update_field)?,
        MigrationStep::DeleteField(delete_field) => apply_delete_field(datamodel, delete_field)?,
        MigrationStep::CreateTypeAlias(create_type_alias) => apply_create_type_alias(datamodel, create_type_alias)?,
        MigrationStep::UpdateTypeAlias(update_type_alias) => apply_update_type_alias(datamodel, update_type_alias)?,
        MigrationStep::DeleteTypeAlias(delete_type_alias) => apply_delete_type_alias(datamodel, delete_type_alias)?,
        MigrationStep::CreateAttribute(create_attribute) => apply_create_attribute(datamodel, create_attribute)?,
        MigrationStep::DeleteAttribute(delete_attribute) => apply_delete_attribute(datamodel, delete_attribute)?,
        MigrationStep::CreateArgument(create_attribute_argument) => {
            apply_create_attribute_argument(datamodel, create_attribute_argument)
        }
        MigrationStep::DeleteArgument(delete_attribute_argument) => {
            apply_delete_attribute_argument(datamodel, delete_attribute_argument)
        }
        MigrationStep::UpdateArgument(update_attribute_argument) => {
            apply_update_attribute_argument(datamodel, update_attribute_argument)
        }
        MigrationStep::CreateSource(create_source) => apply_create_source(datamodel, create_source)?,
        MigrationStep::DeleteSource(delete_source) => apply_delete_source(datamodel, delete_source)?,
    };

    Ok(())
}

fn apply_create_source(datamodel: &mut ast::SchemaAst, step: &CreateSource) -> Result<(), CalculatorError> {
    let steps::CreateSource { source: name } = step;
    if datamodel.find_source(name).is_some() {
        return Err(format_err!(
            "The datasource {} already exists in this Schema. It is not possible to create it once more.",
            name
        )
        .into());
    }

    let new_source = ast::SourceConfig {
        documentation: None,
        name: new_ident(name.clone()),
        span: new_span(),
        properties: Vec::new(),
    };

    datamodel.tops.push(ast::Top::Source(new_source));

    Ok(())
}

fn apply_delete_source(datamodel: &mut ast::SchemaAst, step: &DeleteSource) -> Result<(), CalculatorError> {
    datamodel.find_model(&step.source).ok_or_else(|| {
        format_err!(
            "The source {} does not exist in this Schema. It is not possible to delete it.",
            &step.source
        )
    })?;

    let new_sources = datamodel
        .tops
        .drain(..)
        .filter(|top| match top {
            ast::Top::Source(source) => source.name.name != step.source,
            _ => true,
        })
        .collect();

    datamodel.tops = new_sources;

    Ok(())
}

fn apply_create_enum(datamodel: &mut ast::SchemaAst, step: &steps::CreateEnum) -> Result<(), CalculatorError> {
    let steps::CreateEnum { r#enum: name, values } = step;

    if datamodel.find_enum(&name).is_some() {
        return Err(format_err!(
            "The enum {} already exists in this Datamodel. It is not possible to create it once more.",
            name
        )
        .into());
    }

    let values = values
        .iter()
        .map(|value_name| ast::EnumValue {
            name: Identifier::new(value_name),
            attributes: vec![],
            documentation: None,
            span: new_span(),
            commented_out: false,
        })
        .collect();

    let new_enum = ast::Enum {
        documentation: None,
        name: new_ident(name.clone()),
        span: new_span(),
        values,
        attributes: vec![],
    };

    datamodel.tops.push(ast::Top::Enum(new_enum));

    Ok(())
}

fn apply_create_field(datamodel: &mut ast::SchemaAst, step: &steps::CreateField) -> Result<(), CalculatorError> {
    if datamodel.find_field(&step.model, &step.field).is_some() {
        return Err(format_err!(
            "The field {} on model {} already exists in this Datamodel. It is not possible to create it once more.",
            &step.field,
            &step.model,
        )
        .into());
    }

    let model = datamodel
        .find_model_mut(&step.model)
        .ok_or_else(|| format_err!("CreateField on unknown model: `{}`", step.model))?;

    let steps::CreateField {
        arity,
        model: _,
        field,
        tpe,
    } = step;

    let field = ast::Field {
        arity: arity.into(),
        name: new_ident(field.to_owned()),
        documentation: None,
        field_type: new_ident(tpe.clone()),
        span: new_span(),
        attributes: Vec::new(),
        is_commented_out: false,
    };
    model.fields.push(field);

    Ok(())
}

fn apply_create_model(datamodel: &mut ast::SchemaAst, step: &steps::CreateModel) -> Result<(), CalculatorError> {
    if datamodel.find_model(&step.model).is_some() {
        return Err(format_err!(
            "The model {} already exists in this Datamodel. It is not possible to create it once more.",
            &step.model
        )
        .into());
    }

    let model = ast::Model {
        documentation: None,
        name: new_ident(step.model.clone()),
        span: new_span(),
        fields: vec![],
        attributes: vec![],
        commented_out: false,
    };

    datamodel.tops.push(ast::Top::Model(model));

    Ok(())
}

fn apply_update_model(datamodel: &mut ast::SchemaAst, step: &steps::UpdateModel) -> Result<(), CalculatorError> {
    let model = datamodel.find_model_mut(&step.model).ok_or_else(|| {
        format_err!(
            "The model {} does not exist in this Datamodel. It is not possible to update it.",
            &step.model
        )
    })?;

    apply_model_update(model, &step.new_name, update_model_name);

    Ok(())
}

fn apply_model_update<T, F: Fn(&mut ast::Model, &T)>(model: &mut ast::Model, update: &Option<T>, apply_fn: F) {
    if let Some(update) = update {
        apply_fn(model, update)
    }
}

fn update_model_name(model: &mut ast::Model, new_name: &String) {
    model.name = new_ident(new_name.clone());
}

fn apply_delete_model(datamodel: &mut ast::SchemaAst, step: &steps::DeleteModel) -> Result<(), CalculatorError> {
    datamodel.find_model(&step.model).ok_or_else(|| {
        format_err!(
            "The model {} does not exist in this Datamodel. It is not possible to delete it.",
            &step.model
        )
    })?;

    let new_models = datamodel
        .tops
        .drain(..)
        .filter(|top| match top {
            ast::Top::Model(model) => model.name.name != step.model,
            _ => true,
        })
        .collect();

    datamodel.tops = new_models;

    Ok(())
}

fn apply_update_field(datamodel: &mut ast::SchemaAst, step: &steps::UpdateField) -> Result<(), CalculatorError> {
    if datamodel.find_model(&step.model).is_none() {
        return Err(format_err!(
            "The model {} does not exist in this Datamodel. It is not possible to update a field in it.",
            &step.model
        )
        .into());
    }

    let field = datamodel.find_field_mut(&step.model, &step.field).ok_or_else(|| {
        format_err!(
            "The field {} on model {} does not exist in this Datamodel. It is not possible to update it.",
            &step.field,
            &step.model
        )
    })?;

    apply_field_update(field, &step.arity.map(|x| x.into()), update_field_arity);
    apply_field_update(field, &step.tpe, update_field_type);
    apply_field_update(field, &step.new_name, update_field_name);

    Ok(())
}

fn apply_field_update<T, F: Fn(&mut ast::Field, &T)>(field: &mut ast::Field, update: &Option<T>, apply_fn: F) {
    if let Some(update) = update {
        apply_fn(field, update);
    }
}

fn update_field_arity(field: &mut ast::Field, new_arity: &ast::FieldArity) {
    field.arity = *new_arity;
}

fn update_field_type(field: &mut ast::Field, new_type: &String) {
    field.field_type = new_ident(new_type.clone());
}

fn update_field_name(field: &mut ast::Field, new_name: &String) {
    field.name = new_ident(new_name.clone());
}

fn apply_delete_field(datamodel: &mut ast::SchemaAst, step: &steps::DeleteField) -> Result<(), CalculatorError> {
    datamodel.find_model(&step.model).ok_or_else(|| {
        format_err!(
            "The model {} does not exist in this Datamodel. It is not possible to delete a field in it.",
            &step.model
        )
    })?;

    datamodel.find_field(&step.model, &step.field).ok_or_else(|| {
        format_err!(
            "The field {} on model {} does not exist in this Datamodel. It is not possible to delete it.",
            &step.field,
            &step.model
        )
    })?;

    let model = datamodel.find_model_mut(&step.model).unwrap();

    let new_fields: Vec<_> = model
        .fields
        .drain(..)
        .filter(|field| field.name.name != step.field)
        .collect();

    model.fields = new_fields;

    Ok(())
}

fn apply_update_enum(datamodel: &mut ast::SchemaAst, step: &steps::UpdateEnum) -> Result<(), CalculatorError> {
    let r#enum = datamodel.find_enum_mut(&step.r#enum).ok_or_else(|| {
        format_err!(
            "The enum {} does not exist in this Datamodel. It is not possible to update it.",
            &step.r#enum
        )
    })?;

    apply_enum_update(r#enum, &step.new_name, update_enum_name);
    add_enum_values(r#enum, &step.created_values);
    remove_enum_values(r#enum, &step.deleted_values);

    Ok(())
}

fn apply_enum_update<T, F: Fn(&mut ast::Enum, &T)>(r#enum: &mut ast::Enum, update: &Option<T>, apply_fn: F) {
    if let Some(update) = update {
        apply_fn(r#enum, update);
    }
}

fn update_enum_name(r#enum: &mut ast::Enum, new_name: &String) {
    r#enum.name = new_ident(new_name.clone());
}

fn add_enum_values(r#enum: &mut ast::Enum, added_values: &[String]) {
    r#enum
        .values
        .extend(added_values.iter().map(|added_name| ast::EnumValue {
            name: Identifier::new(added_name),
            attributes: vec![],
            documentation: None,
            span: new_span(),
            commented_out: false,
        }))
}

fn remove_enum_values(r#enum: &mut ast::Enum, removed_values: &[String]) {
    let new_values = r#enum
        .values
        .drain(..)
        .filter(|value| {
            removed_values
                .iter()
                .find(|removed_value| removed_value.as_str() == value.name.name.as_str())
                .is_none()
        })
        .collect();

    r#enum.values = new_values;
}

fn apply_delete_enum(datamodel: &mut ast::SchemaAst, step: &steps::DeleteEnum) -> Result<(), CalculatorError> {
    datamodel.find_enum(&step.r#enum).ok_or_else(|| {
        format_err!(
            "The enum {} does not exist in this Datamodel. It is not possible to delete it.",
            &step.r#enum
        )
    })?;

    let new_tops = datamodel
        .tops
        .drain(..)
        .filter(|top| match top {
            ast::Top::Enum(r#enum) => r#enum.name.name != step.r#enum,
            _ => true,
        })
        .collect();

    datamodel.tops = new_tops;

    Ok(())
}

fn apply_create_attribute(
    datamodel: &mut ast::SchemaAst,
    step: &steps::CreateAttribute,
) -> Result<(), CalculatorError> {
    let attributes = find_attributes_mut(datamodel, &step.location.path)
        .ok_or_else(|| format_err!("CreateAttribute on absent target: {:?}.", step))?;

    let new_attribute = ast::Attribute {
        name: new_ident(step.location.attribute.clone()),
        arguments: step
            .location
            .path
            .arguments()
            .as_ref()
            .map(|args| args.iter().map(|arg| arg.into()).collect())
            .unwrap_or_else(Vec::new),
        span: new_span(),
    };

    attributes.push(new_attribute);

    Ok(())
}

fn apply_delete_attribute(
    datamodel: &mut ast::SchemaAst,
    step: &steps::DeleteAttribute,
) -> Result<(), CalculatorError> {
    let attributes = find_attributes_mut(datamodel, &step.location.path)
        .ok_or_else(|| format_err!("DeleteAttribute on absent target: {:?}.", step))?;

    let new_attributes = attributes
        .drain(..)
        .filter(|attribute| !step.location.matches_ast_attribute(attribute))
        .collect();

    *attributes = new_attributes;

    Ok(())
}

fn apply_create_attribute_argument(datamodel: &mut ast::SchemaAst, step: &steps::CreateArgument) {
    let mut argument_container = find_argument_container(datamodel, &step.location).unwrap();

    argument_container.arguments().push(ast::Argument {
        name: new_ident(step.argument.clone()),
        span: new_span(),
        value: step.value.to_ast_expression(),
    });
}

fn apply_update_attribute_argument(datamodel: &mut ast::SchemaAst, step: &steps::UpdateArgument) {
    let mut argument_container = find_argument_container(datamodel, &step.location).unwrap();

    for argument in argument_container.arguments().iter_mut() {
        if argument.name.name == step.argument {
            argument.value = step.new_value.to_ast_expression();
        }
    }
}

fn apply_delete_attribute_argument(datamodel: &mut ast::SchemaAst, step: &steps::DeleteArgument) {
    let mut argument_container = find_argument_container(datamodel, &step.location).unwrap();

    let new_arguments = argument_container
        .arguments()
        .drain(..)
        .filter(|arg| arg.name.name != step.argument)
        .collect();

    argument_container.set_arguments(new_arguments)
}

fn apply_create_type_alias(
    datamodel: &mut ast::SchemaAst,
    step: &steps::CreateTypeAlias,
) -> Result<(), CalculatorError> {
    if datamodel.find_type_alias(&step.type_alias).is_some() {
        return Err(format_err!(
            "The type {} already exists in this Datamodel. It is not possible to create it once more.",
            &step.type_alias
        )
        .into());
    }

    let type_alias = ast::Field {
        documentation: None,
        name: new_ident(step.type_alias.clone()),
        span: new_span(),
        arity: step.arity.into(),
        attributes: vec![],
        field_type: new_ident(step.r#type.clone()),
        is_commented_out: false,
    };

    datamodel.tops.push(ast::Top::Type(type_alias));

    Ok(())
}

fn apply_update_type_alias(
    datamodel: &mut ast::SchemaAst,
    step: &steps::UpdateTypeAlias,
) -> Result<(), CalculatorError> {
    let type_alias = datamodel
        .find_type_alias_mut(&step.type_alias)
        .ok_or_else(|| format_err!("UpdateTypeAlias on unknown custom type `{}`", &step.type_alias))?;

    if let Some(r#type) = step.r#type.as_ref() {
        type_alias.field_type = new_ident(r#type.clone())
    }

    Ok(())
}

fn apply_delete_type_alias(
    datamodel: &mut ast::SchemaAst,
    step: &steps::DeleteTypeAlias,
) -> Result<(), CalculatorError> {
    datamodel.find_type_alias(&step.type_alias).ok_or_else(|| {
        format_err!(
            "The type {} does not exist in this Datamodel. It is not possible to delete it.",
            &step.type_alias
        )
    })?;

    let new_tops = datamodel
        .tops
        .drain(..)
        .filter(|top| match top {
            ast::Top::Type(field) => field.name.name != step.type_alias,
            _ => true,
        })
        .collect();

    datamodel.tops = new_tops;

    Ok(())
}

fn new_ident(name: String) -> ast::Identifier {
    ast::Identifier { name, span: new_span() }
}

fn new_span() -> ast::Span {
    ast::Span::empty()
}

fn find_argument_container<'schema>(
    datamodel: &'schema mut ast::SchemaAst,
    locator: &steps::ArgumentLocation,
) -> Option<ArgumentContainer<'schema>> {
    match locator {
        steps::ArgumentLocation::Source(source_location) => datamodel
            .find_source_mut(&source_location.source)
            .map(|sc| ArgumentContainer::SourceConfig(sc)),
        steps::ArgumentLocation::Attribute(attribute_location) => {
            find_attribute_mut(datamodel, attribute_location).map(|d| ArgumentContainer::Attribute(d))
        }
    }
}

fn find_attribute_mut<'a>(
    datamodel: &'a mut ast::SchemaAst,
    locator: &steps::AttributeLocation,
) -> Option<&'a mut ast::Attribute> {
    find_attributes_mut(datamodel, &locator.path)?
        .iter_mut()
        .find(|attribute| attribute.name.name == locator.attribute)
}

fn find_attributes_mut<'a>(
    datamodel: &'a mut ast::SchemaAst,
    location: &steps::AttributePath,
) -> Option<&'a mut Vec<ast::Attribute>> {
    let attributes = match location {
        steps::AttributePath::Field { model, field } => &mut datamodel.find_field_mut(&model, &field)?.attributes,
        steps::AttributePath::Model { model, arguments: _ } => &mut datamodel.find_model_mut(&model)?.attributes,
        steps::AttributePath::Enum { r#enum } => &mut datamodel.find_enum_mut(&r#enum)?.attributes,
        steps::AttributePath::EnumValue { r#enum, value } => {
            let enum_struct = datamodel.find_enum_mut(&r#enum)?;
            let value = enum_struct
                .values
                .iter_mut()
                .find(|value_struct| &value_struct.name.name == value)?;

            &mut value.attributes
        }
        steps::AttributePath::TypeAlias { type_alias } => &mut datamodel.find_type_alias_mut(&type_alias)?.attributes,
    };

    Some(attributes)
}

use datamodel::ast::{self, SchemaAst};
use failure::{format_err, Fail};
use migration_connector::steps::{self, MigrationStep};

pub trait DataModelCalculator: Send + Sync + 'static {
    fn infer(&self, current: &SchemaAst, steps: &[MigrationStep]) -> Result<SchemaAst, CalculatorError>;
}

#[derive(Debug, Fail)]
#[fail(display = "{}", _0)]
pub struct CalculatorError(#[fail(cause)] failure::Error);

impl From<failure::Error> for CalculatorError {
    fn from(fe: failure::Error) -> Self {
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
        MigrationStep::CreateCustomType(create_custom_type) => apply_create_custom_type(datamodel, create_custom_type)?,
        MigrationStep::DeleteCustomType(delete_custom_type) => apply_delete_custom_type(datamodel, delete_custom_type)?,
        MigrationStep::CreateDirective(create_directive) => apply_create_directive(datamodel, create_directive)?,
        MigrationStep::DeleteDirective(delete_directive) => apply_delete_directive(datamodel, delete_directive)?,
        MigrationStep::CreateDirectiveArgument(create_directive_argument) => {
            apply_create_directive_argument(datamodel, create_directive_argument)
        }
        MigrationStep::DeleteDirectiveArgument(delete_directive_argument) => {
            apply_delete_directive_argument(datamodel, delete_directive_argument)
        }
        MigrationStep::UpdateDirectiveArgument(update_directive_argument) => {
            apply_update_directive_argument(datamodel, update_directive_argument)
        }
    };

    Ok(())
}

fn apply_create_enum(datamodel: &mut ast::SchemaAst, step: &steps::CreateEnum) -> Result<(), CalculatorError> {
    let steps::CreateEnum { r#enum: name, values } = step;

    if let Some(_) = datamodel.find_enum(&name) {
        return Err(format_err!(
            "The enum {} already exists in this Datamodel. It is not possible to create it once more.",
            name
        )
        .into());
    }

    let values = values
        .iter()
        .map(|value_name| ast::EnumValue {
            name: value_name.clone(),
            span: new_span(),
        })
        .collect();

    let new_enum = ast::Enum {
        documentation: None,
        name: new_ident(name.clone()),
        span: new_span(),
        values,
        directives: vec![],
    };

    datamodel.tops.push(ast::Top::Enum(new_enum));

    Ok(())
}

fn apply_create_field(datamodel: &mut ast::SchemaAst, step: &steps::CreateField) -> Result<(), CalculatorError> {
    if let Some(_) = datamodel.find_field(&step.model, &step.field) {
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
        arity: arity.clone(),
        name: new_ident(field.to_owned()),
        documentation: None,
        field_type: new_ident(tpe.clone()),
        span: new_span(),
        directives: Vec::new(),
        default_value: None,
    };
    model.fields.push(field);

    Ok(())
}

fn apply_create_model(datamodel: &mut ast::SchemaAst, step: &steps::CreateModel) -> Result<(), CalculatorError> {
    if let Some(_) = datamodel.find_model(&step.model) {
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
        directives: vec![],
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
    if let None = datamodel.find_model(&step.model) {
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

    apply_field_update(field, &step.arity, update_field_arity);
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
    field.arity = new_arity.clone();
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
            name: added_name.clone(),
            span: new_span(),
        }))
}

fn remove_enum_values(r#enum: &mut ast::Enum, removed_values: &[String]) {
    let new_values = r#enum
        .values
        .drain(..)
        .filter(|value| {
            removed_values
                .iter()
                .find(|removed_value| removed_value.as_str() == value.name.as_str())
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

fn apply_create_directive(
    datamodel: &mut ast::SchemaAst,
    step: &steps::CreateDirective,
) -> Result<(), CalculatorError> {
    let directives = find_directives_mut(datamodel, &step.locator.location)
        .ok_or_else(|| format_err!("CreateDirective on absent target: {:?}.", step))?;

    let new_directive = ast::Directive {
        name: new_ident(step.locator.directive.clone()),
        arguments: step
            .locator
            .arguments
            .as_ref()
            .map(|args| args.iter().map(|arg| arg.into()).collect())
            .unwrap_or_else(Vec::new),
        span: new_span(),
    };

    directives.push(new_directive);

    Ok(())
}

fn apply_delete_directive(
    datamodel: &mut ast::SchemaAst,
    step: &steps::DeleteDirective,
) -> Result<(), CalculatorError> {
    let directives = find_directives_mut(datamodel, &step.locator.location)
        .ok_or_else(|| format_err!("DeleteDirective on absent target: {:?}.", step))?;

    let new_directives = directives
        .drain(..)
        .filter(|directive| !step.locator.matches_ast_directive(directive))
        .collect();

    *directives = new_directives;

    Ok(())
}

fn apply_create_directive_argument(datamodel: &mut ast::SchemaAst, step: &steps::CreateDirectiveArgument) {
    let directive = find_directive_mut(datamodel, &step.directive_location).unwrap();

    directive.arguments.push(ast::Argument {
        name: new_ident(step.argument.clone()),
        span: new_span(),
        value: step.value.to_ast_expression(),
    });
}

fn apply_update_directive_argument(datamodel: &mut ast::SchemaAst, step: &steps::UpdateDirectiveArgument) {
    let directive = find_directive_mut(datamodel, &step.directive_location).unwrap();

    for argument in directive.arguments.iter_mut() {
        if argument.name.name == step.argument {
            argument.value = step.new_value.to_ast_expression();
        }
    }
}

fn apply_delete_directive_argument(datamodel: &mut ast::SchemaAst, step: &steps::DeleteDirectiveArgument) {
    let directive = find_directive_mut(datamodel, &step.directive_location).unwrap();

    let new_arguments = directive
        .arguments
        .drain(..)
        .filter(|arg| arg.name.name != step.argument)
        .collect();

    directive.arguments = new_arguments;
}

fn apply_create_custom_type(
    datamodel: &mut ast::SchemaAst,
    step: &steps::CreateCustomType,
) -> Result<(), CalculatorError> {
    if let Some(_) = datamodel.find_custom_type(&step.custom_type) {
        return Err(format_err!(
            "The type {} already exists in this Datamodel. It is not possible to create it once more.",
            &step.custom_type
        )
        .into());
    }

    let custom_type = ast::Field {
        documentation: None,
        name: new_ident(step.custom_type.clone()),
        span: new_span(),
        default_value: None,
        arity: step.arity.clone(),
        directives: vec![],
        field_type: new_ident(step.r#type.clone()),
    };

    datamodel.tops.push(ast::Top::Type(custom_type));

    Ok(())
}

fn apply_delete_custom_type(
    datamodel: &mut ast::SchemaAst,
    step: &steps::DeleteCustomType,
) -> Result<(), CalculatorError> {
    datamodel.find_custom_type(&step.custom_type).ok_or_else(|| {
        format_err!(
            "The type {} does not exist in this Datamodel. It is not possible to delete it.",
            &step.custom_type
        )
    })?;

    let new_tops = datamodel
        .tops
        .drain(..)
        .filter(|top| match top {
            ast::Top::Type(field) => field.name.name != step.custom_type,
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

fn find_directives_mut<'a>(
    datamodel: &'a mut ast::SchemaAst,
    location: &steps::DirectiveType,
) -> Option<&'a mut Vec<ast::Directive>> {
    let directives = match location {
        steps::DirectiveType::Field { model, field } => &mut datamodel.find_field_mut(&model, &field)?.directives,
        steps::DirectiveType::Model { model } => &mut datamodel.find_model_mut(&model)?.directives,
        steps::DirectiveType::Enum { r#enum } => &mut datamodel.find_enum_mut(&r#enum)?.directives,
        steps::DirectiveType::CustomType { custom_type } => {
            &mut datamodel.find_custom_type_mut(&custom_type)?.directives
        }
    };

    Some(directives)
}

fn find_directive_mut<'a>(
    datamodel: &'a mut ast::SchemaAst,
    locator: &steps::DirectiveLocation,
) -> Option<&'a mut ast::Directive> {
    find_directives_mut(datamodel, &locator.location)?
        .iter_mut()
        .find(|directive| directive.name.name == locator.directive)
}

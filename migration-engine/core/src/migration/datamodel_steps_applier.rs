use datamodel::{ast, parse_to_ast};
use failure::format_err;
use migration_connector::ast_steps::{self as steps, MigrationStep};

pub(crate) fn apply(initial_datamodel: &str, steps: &[MigrationStep]) -> crate::Result<ast::Datamodel> {
    let mut datamodel = parse_to_ast(initial_datamodel)?;

    for step in steps {
        apply_step(&mut datamodel, step);
    }

    Ok(datamodel)
}

fn apply_step(datamodel: &mut ast::Datamodel, step: &MigrationStep) {
    match step {
        MigrationStep::CreateEnum(create_enum) => apply_create_enum(datamodel, create_enum),
        MigrationStep::UpdateEnum(update_enum) => apply_update_enum(datamodel, update_enum),
        MigrationStep::DeleteEnum(delete_enum) => apply_delete_enum(datamodel, delete_enum),
        MigrationStep::CreateModel(create_model) => apply_create_model(datamodel, create_model),
        MigrationStep::UpdateModel(update_model) => apply_update_model(datamodel, update_model),
        MigrationStep::DeleteModel(delete_model) => apply_delete_model(datamodel, delete_model),
        MigrationStep::CreateField(create_field) => apply_create_field(datamodel, create_field),
        MigrationStep::UpdateField(update_field) => apply_update_field(datamodel, update_field),
        MigrationStep::DeleteField(delete_field) => apply_delete_field(datamodel, delete_field),
    }
}

fn apply_create_enum(datamodel: &mut ast::Datamodel, step: &steps::CreateEnum) {
    let steps::CreateEnum { name, values } = step;

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

    datamodel.models.push(ast::Top::Enum(new_enum));
}

fn apply_create_field(datamodel: &mut ast::Datamodel, step: &steps::CreateField) {
    let model = find_model_mut(datamodel, &step.model)
        .ok_or_else(|| format_err!("CreateField on unknown model: `{}`", step.model))
        .unwrap();

    let steps::CreateField {
        arity,
        db_name,
        model: _,
        name,
        tpe,
        default,
    } = step;

    let mut directives = Vec::new();

    if let Some(db_name) = db_name {
        directives.push(new_map_directive(db_name.to_owned()))
    };

    let field = ast::Field {
        arity: arity.clone(),
        name: new_ident(name.to_owned()),
        documentation: None,
        field_type: new_ident(tpe.clone()),
        span: new_span(),
        directives: Vec::new(),
        default_value: None,
    };
    model.fields.push(field);
}

fn apply_create_model(datamodel: &mut ast::Datamodel, step: &steps::CreateModel) {
    // TODO: steps.db_name

    let model = ast::Model {
        documentation: None,
        name: new_ident(step.name.clone()),
        span: new_span(),
        fields: vec![],
        directives: vec![],
    };

    datamodel.models.push(ast::Top::Model(model));
}

fn apply_update_model(datamodel: &mut ast::Datamodel, step: &steps::UpdateModel) {
    let model = find_model_mut(datamodel, &step.name)
        .ok_or_else(|| format_err!("UpdateModel on unknown model: `{}`", &step.name))
        .unwrap();

    apply_model_update(model, &step.new_name, update_model_name);
}

fn apply_model_update<T, F: Fn(&mut ast::Model, &T)>(model: &mut ast::Model, update: &Option<T>, apply_fn: F) {
    if let Some(update) = update {
        apply_fn(model, update)
    }
}

fn update_model_name(model: &mut ast::Model, new_name: &String) {
    model.name = new_ident(new_name.clone());
}

fn apply_delete_model(datamodel: &mut ast::Datamodel, step: &steps::DeleteModel) {
    let new_models = datamodel
        .models
        .drain(..)
        .filter(|top| match top {
            ast::Top::Model(model) => model.name.name != step.name,
            _ => true,
        })
        .collect();

    datamodel.models = new_models;
}

fn apply_update_field(datamodel: &mut ast::Datamodel, step: &steps::UpdateField) {
    let field = find_model_field_mut(datamodel, &step.model, &step.name)
        .ok_or_else(|| format_err!("UpdateStep on unknown field: `{}.{}`.", &step.model, &step.name))
        .unwrap();

    apply_field_update(field, &step.arity, update_field_arity);
    apply_field_update(field, &step.tpe, update_field_type);
    apply_field_update(field, &step.new_name, update_field_name);
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

fn apply_delete_field(datamodel: &mut ast::Datamodel, step: &steps::DeleteField) {
    let model = find_model_mut(datamodel, &step.model)
        .ok_or_else(|| format_err!("DeleteField on unknown model: `{}`.", &step.model))
        .unwrap();

    let previous_fields_len = model.fields.len();

    let new_fields: Vec<_> = model
        .fields
        .drain(..)
        .filter(|field| field.name.name != step.name)
        .collect();

    let new_fields_len = new_fields.len();

    debug_assert_eq!(new_fields_len, previous_fields_len - 1);

    model.fields = new_fields;
}

fn apply_update_enum(datamodel: &mut ast::Datamodel, step: &steps::UpdateEnum) {
    let r#enum = find_enum_mut(datamodel, &step.name)
        .ok_or_else(|| format_err!("UpdateEnum on unknown enum: `{}`.", &step.name))
        .unwrap();

    apply_enum_update(r#enum, &step.new_name, update_enum_name);
    apply_enum_update(r#enum, &step.created_values, add_enum_values);
    apply_enum_update(r#enum, &step.deleted_values, remove_enum_values);
}

fn apply_enum_update<T, F: Fn(&mut ast::Enum, &T)>(r#enum: &mut ast::Enum, update: &Option<T>, apply_fn: F) {
    if let Some(update) = update {
        apply_fn(r#enum, update);
    }
}

fn update_enum_name(r#enum: &mut ast::Enum, new_name: &String) {
    r#enum.name = new_ident(new_name.clone());
}

fn add_enum_values(r#enum: &mut ast::Enum, added_values: &Vec<String>) {
    r#enum
        .values
        .extend(added_values.iter().map(|added_name| ast::EnumValue {
            name: added_name.clone(),
            span: new_span(),
        }))
}

fn remove_enum_values(r#enum: &mut ast::Enum, removed_values: &Vec<String>) {
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

fn apply_delete_enum(datamodel: &mut ast::Datamodel, step: &steps::DeleteEnum) {
    let new_tops = datamodel
        .models
        .drain(..)
        .filter(|top| match top {
            ast::Top::Enum(r#enum) => r#enum.name.name != step.name,
            _ => true,
        })
        .collect();

    datamodel.models = new_tops;
}

fn new_ident(name: String) -> ast::Identifier {
    ast::Identifier { name, span: new_span() }
}

fn new_span() -> ast::Span {
    ast::Span::empty()
}

/// See [the spec](https://github.com/prisma/specs/tree/master/schema#map_-name-string).
fn new_map_directive(name: String) -> ast::Directive {
    ast::Directive {
        name: new_ident("map".to_owned()),
        span: new_span(),
        arguments: vec![ast::Argument {
            name: new_ident("name".to_owned()),
            span: new_span(),
            value: ast::Value::StringValue(name.to_owned(), new_span()),
        }],
    }
}

fn find_model_mut<'a>(datamodel: &'a mut ast::Datamodel, model_name: &str) -> Option<&'a mut ast::Model> {
    datamodel.models.iter_mut().find_map(|top| match top {
        ast::Top::Model(model) if model.name.name == model_name => Some(model),
        _ => None,
    })
}

fn find_enum_mut<'a>(datamodel: &'a mut ast::Datamodel, enum_name: &str) -> Option<&'a mut ast::Enum> {
    datamodel.models.iter_mut().find_map(|top| match top {
        ast::Top::Enum(r#enum) if r#enum.name.name == enum_name => Some(r#enum),
        _ => None,
    })
}

fn find_model_field_mut<'a>(
    datamodel: &'a mut ast::Datamodel,
    model_name: &str,
    field_name: &str,
) -> Option<&'a mut ast::Field> {
    find_model_mut(datamodel, model_name)
        .and_then(|model| model.fields.iter_mut().find(|field| field.name.name == field_name))
}

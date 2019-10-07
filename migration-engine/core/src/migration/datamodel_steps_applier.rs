use datamodel::{ast, parse_to_ast};
use migration_connector::{steps, MigrationStep};

pub(crate) fn apply(initial: &str, steps: &[MigrationStep]) -> crate::Result<ast::Datamodel> {
    let mut datamodel = parse_to_ast(initial)?;

    for step in steps {
        apply_step(&mut datamodel, step);
    }

    Ok(datamodel)
}

fn apply_step(datamodel: &mut ast::Datamodel, step: &MigrationStep) {
    match step {
        MigrationStep::CreateEnum(create_enum) => apply_create_enum(datamodel, create_enum),
        MigrationStep::CreateField(create_field) => apply_create_field(datamodel, create_field),
        MigrationStep::DeleteModel(delete_model) => apply_delete_model(datamodel, delete_model),
        _ => unimplemented!("Migration step: {:?}", step),
    }
}

fn apply_create_enum(datamodel: &mut ast::Datamodel, step: &steps::CreateEnum) {
    let steps::CreateEnum { name, db_name, values } = step;

    let directives = if let Some(db_name) = db_name {
        vec![new_map_directive(db_name.to_owned())]
    } else {
        vec![]
    };

    let new_enum = ast::Enum {
        documentation: None,
        name: new_ident(name.to_owned()),
        span: new_span(),
        values: values
            .iter()
            .map(|value_name| ast::EnumValue {
                name: value_name.into(),
                span: new_span(),
            })
            .collect(),
        directives,
    };

    datamodel.models.push(ast::Top::Enum(new_enum));
}

fn apply_create_field(datamodel: &mut ast::Datamodel, step: &steps::CreateField) {
    let model = find_model_mut(datamodel, &step.model).expect("CreateField on unknown model");
    let steps::CreateField {
        arity,
        db_name,
        default,
        id,
        is_created_at: _,
        is_unique: _,
        is_updated_at: _,
        model: _,
        name,
        scalar_list,
        tpe,
    } = step;

    let mut directives = Vec::new();

    if let Some(db_name) = db_name {
        directives.push(new_map_directive(db_name.to_owned()))
    };

    if let Some(id_info) = id {
        unimplemented!("id info");
    }

    let field = ast::Field {
        arity: unimplemented!(),
        name: new_ident(name.to_owned()),
        documentation: None,
        field_type: unimplemented!("display dml field type"),
        span: new_span(),
        directives: vec![unimplemented!()],
        default_value: unimplemented!(),
    };
    model.fields.push(field);
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
        ast::Top::Model(model) => Some(model),
        _ => None,
    })
}

#![deny(rust_2018_idioms)]

mod directives;
mod enums;
mod fields;
mod models;
mod top_level;
mod values;

use enums::EnumDiffer;
use fields::FieldDiffer;
use models::ModelDiffer;
use top_level::TopDiffer;

use datamodel::ast;
use migration_connector::ast_steps::{self as steps, MigrationStep};

pub(crate) fn diff(previous: &ast::Datamodel, next: &ast::Datamodel) -> Vec<MigrationStep> {
    DatamodelDiffer::new().diff(previous, next)
}

#[derive(Default)]
struct DatamodelDiffer {
    steps: Vec<MigrationStep>,
}

impl DatamodelDiffer {
    fn new() -> Self {
        Self::default()
    }

    fn diff(mut self, previous: &ast::Datamodel, next: &ast::Datamodel) -> Vec<MigrationStep> {
        let differ = TopDiffer { previous, next };

        self.push_enums(&differ);
        self.push_models(&differ);

        self.steps
    }

    fn push_enums(&mut self, differ: &TopDiffer<'_>) {
        self.push_created_enums(differ.created_enums());
        self.push_deleted_enums(differ.deleted_enums());
        self.push_updated_enums(differ.enum_pairs());
    }

    fn push_created_enums<'a>(&mut self, enums: impl Iterator<Item = &'a ast::Enum>) {
        let created_enum_steps = enums
            .map(|new_enum| steps::CreateEnum {
                name: new_enum.name.name.clone(),
                values: new_enum.values.iter().map(|value| value.name.clone()).collect(),
            })
            .map(MigrationStep::CreateEnum);

        self.steps.extend(created_enum_steps);
    }

    fn push_deleted_enums<'a>(&mut self, enums: impl Iterator<Item = &'a ast::Enum>) {
        let deleted_enum_steps = enums
            .map(|deleted_enum| steps::DeleteEnum {
                name: deleted_enum.name.name.clone(),
            })
            .map(MigrationStep::DeleteEnum);

        self.steps.extend(deleted_enum_steps)
    }

    fn push_updated_enums<'a>(&mut self, enums: impl Iterator<Item = EnumDiffer<'a>>) {
        self.steps.extend(enums.filter(|enm| enm.values_changed()).map(|enm| {
            let created_values: Vec<_> = enm.created_values().map(|value| value.name.to_owned()).collect();
            let deleted_values: Vec<_> = enm.deleted_values().map(|value| value.name.to_owned()).collect();

            MigrationStep::UpdateEnum(steps::UpdateEnum {
                name: enm.previous.name.name.clone(),
                new_name: diff_value(&enm.previous.name.name, &enm.next.name.name),
                created_values: Some(created_values).filter(Vec::is_empty),
                deleted_values: Some(deleted_values).filter(Vec::is_empty),
            })
        }))
    }

    fn push_models(&mut self, differ: &TopDiffer<'_>) {
        self.push_created_models(differ.created_models());
        self.push_deleted_models(differ.deleted_models());
        self.push_updated_models(differ.model_pairs());
    }

    fn push_created_models<'a>(&mut self, models: impl Iterator<Item = &'a ast::Model>) {
        for created_model in models {
            let db_name = directives::get_directive_string_value("map", &created_model.directives)
                .map(|db_name| db_name.to_owned());
            let create_model_step = steps::CreateModel {
                name: created_model.name.name.clone(),
                embedded: false, // not represented in the AST yet
                db_name,
            };

            self.steps.push(MigrationStep::CreateModel(create_model_step));

            self.push_created_fields(&created_model.name.name, created_model.fields.iter());

            // TODO: create the directives on that model
        }
    }

    fn push_deleted_models<'a>(&mut self, models: impl Iterator<Item = &'a ast::Model>) {
        let delete_model_steps = models
            .map(|deleted_model| steps::DeleteModel {
                name: deleted_model.name.name.clone(),
            })
            .map(MigrationStep::DeleteModel);
        self.steps.extend(delete_model_steps);
    }

    fn push_updated_models<'a>(&mut self, models: impl Iterator<Item = ModelDiffer<'a>>) {
        models.for_each(|model| {
            let model_name = &model.previous.name.name;
            self.push_created_fields(model_name, model.created_fields());
            self.push_deleted_fields(model_name, model.deleted_fields());
            self.push_updated_fields(model_name, model.field_pairs());
        });
    }

    fn push_created_fields<'a>(&mut self, model_name: &'a str, fields: impl Iterator<Item = &'a ast::Field>) {
        let create_field_steps = fields
            .map(|field| steps::CreateField {
                arity: field.arity,
                name: field.name.name.clone(),
                tpe: field.field_type.name.clone(),
                model: model_name.to_owned(),
                db_name: directives::get_directive_string_value("map", &field.directives).map(String::from),
                default: directives::get_directive_value("default", &field.directives).map(|val| val.to_string()),
            })
            .map(MigrationStep::CreateField);

        self.steps.extend(create_field_steps)
    }

    fn push_deleted_fields<'a>(&mut self, model_name: &'a str, fields: impl Iterator<Item = &'a ast::Field>) {
        let delete_field_steps = fields
            .map(|deleted_field| steps::DeleteField {
                model: model_name.to_owned(),
                name: deleted_field.name.name.clone(),
            })
            .map(MigrationStep::DeleteField);

        self.steps.extend(delete_field_steps);
    }

    fn push_updated_fields<'a>(&mut self, model_name: &'a str, fields: impl Iterator<Item = FieldDiffer<'a>>) {
        for field in fields {
            let update_field_step = steps::UpdateField {
                arity: diff_value(&field.previous.arity, &field.next.arity),
                new_name: diff_value(&field.previous.name.name, &field.next.name.name),
                model: model_name.to_owned(),
                name: field.previous.name.name.clone(),
                tpe: diff_value(&field.previous.field_type.name, &field.next.field_type.name),
            };

            self.steps.push(MigrationStep::UpdateField(update_field_step));
        }
    }
}

fn diff_value<T: PartialEq + Clone>(current: &T, updated: &T) -> Option<T> {
    if current == updated {
        None
    } else {
        Some(updated.clone())
    }
}

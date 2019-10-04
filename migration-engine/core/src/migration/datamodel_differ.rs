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
use migration_connector::{steps, MigrationStep};

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
        self.steps.extend(enums.map(|new_enum| {
            MigrationStep::CreateEnum(steps::CreateEnum {
                name: new_enum.name.name.clone(),
                db_name: directives::get_directive_string_value("map", &new_enum.directives)
                    .map(|db_name| db_name.to_owned()),
                values: new_enum.values.iter().map(|value| value.name.clone()).collect(),
            })
        }));
    }

    fn push_deleted_enums<'a>(&mut self, enums: impl Iterator<Item = &'a ast::Enum>) {
        self.steps.extend(enums.map(|deleted_enum| {
            MigrationStep::DeleteEnum(steps::DeleteEnum {
                name: deleted_enum.name.name.clone(),
            })
        }))
    }

    fn push_updated_enums<'a>(&mut self, enums: impl Iterator<Item = EnumDiffer<'a>>) {
        self.steps.extend(enums.filter(|enm| enm.values_changed()).map(|enm| {
            MigrationStep::UpdateEnum(steps::UpdateEnum {
                name: enm.previous.name.name.clone(),
                new_name: None, // TODO: figure out the proper way to update here
                db_name: None,  // TODO: figure out the proper way to update here
                values: Some(enm.next.values.iter().map(|value| value.name.clone()).collect()),
            })
        }))
    }

    fn push_models(&mut self, differ: &TopDiffer<'_>) {
        self.push_created_models(differ.created_models());
        self.push_deleted_models(differ.deleted_models());
        self.push_updated_models(differ.model_pairs());
    }

    fn push_created_models<'a>(&mut self, models: impl Iterator<Item = &'a ast::Model>) {
        self.steps.extend(models.map(|created_model| {
            MigrationStep::CreateModel(steps::CreateModel {
                name: created_model.name.name.clone(),
                embedded: false, // not represented in the AST yet
                db_name: directives::get_directive_string_value("map", &created_model.directives)
                    .map(|db_name| db_name.to_owned()),
            })
        }))

        // TODO: create the fields and indexes on that model
    }

    fn push_deleted_models<'a>(&mut self, models: impl Iterator<Item = &'a ast::Model>) {
        self.steps.extend(models.map(|deleted_model| {
            MigrationStep::DeleteModel(steps::DeleteModel {
                name: deleted_model.name.name.clone(),
            })
        }))
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
        self.steps
            .extend(fields.map(|field| MigrationStep::CreateField(unimplemented!("CreateField"))))
    }

    fn push_deleted_fields<'a>(&mut self, model_name: &'a str, fields: impl Iterator<Item = &'a ast::Field>) {
        self.steps.extend(fields.map(|deleted_field| {
            MigrationStep::DeleteField(steps::DeleteField {
                model: model_name.to_owned(),
                name: deleted_field.name.name.clone(),
            })
        }))
    }

    fn push_updated_fields<'a>(&mut self, model_name: &'a str, fields: impl Iterator<Item = FieldDiffer<'a>>) {
        fields.for_each(|field| {
            unimplemented!("field updates");
        })
    }
}

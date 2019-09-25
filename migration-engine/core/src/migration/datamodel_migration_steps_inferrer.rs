use datamodel::*;
use migration_connector::steps::*;

pub trait DataModelMigrationStepsInferrer: Send + Sync + 'static {
    fn infer(&self, previous: &Datamodel, next: &Datamodel) -> Vec<MigrationStep>;
}

pub struct DataModelMigrationStepsInferrerImplWrapper {}

impl DataModelMigrationStepsInferrer for DataModelMigrationStepsInferrerImplWrapper {
    fn infer(&self, previous: &Datamodel, next: &Datamodel) -> Vec<MigrationStep> {
        let inferrer = DataModelMigrationStepsInferrerImpl { previous, next };
        inferrer.infer_internal()
    }
}

#[allow(dead_code)]
pub struct DataModelMigrationStepsInferrerImpl<'a> {
    previous: &'a Datamodel,
    next: &'a Datamodel,
}

// TODO: this does not deal with renames yet
impl<'a> DataModelMigrationStepsInferrerImpl<'a> {
    fn infer_internal(&self) -> Vec<MigrationStep> {
        let mut result: Vec<MigrationStep> = Vec::new();
        let models_to_create = self.models_to_create();
        let models_to_delete = self.models_to_delete();
        let models_to_update = self.models_to_update();
        let fields_to_create = self.fields_to_create();
        let fields_to_delete = self.fields_to_delete(&models_to_delete);
        let fields_to_update = self.fields_to_update();
        let enums_to_create = self.enums_to_create();
        let enums_to_delete = self.enums_to_delete();
        let enums_to_update = self.enums_to_update();
        let indexes_to_rename = self.indexes_to_rename();
        let indexes_to_create = self.indexes_to_create(&indexes_to_rename);
        let indexes_to_delete = self.indexes_to_delete(&indexes_to_rename);

        result.append(&mut Self::wrap_as_step(models_to_create, MigrationStep::CreateModel));
        result.append(&mut Self::wrap_as_step(models_to_delete, MigrationStep::DeleteModel));
        result.append(&mut Self::wrap_as_step(models_to_update, MigrationStep::UpdateModel));
        result.append(&mut Self::wrap_as_step(fields_to_create, MigrationStep::CreateField));
        result.append(&mut Self::wrap_as_step(fields_to_delete, MigrationStep::DeleteField));
        result.append(&mut Self::wrap_as_step(fields_to_update, MigrationStep::UpdateField));
        result.append(&mut Self::wrap_as_step(enums_to_create, MigrationStep::CreateEnum));
        result.append(&mut Self::wrap_as_step(enums_to_delete, MigrationStep::DeleteEnum));
        result.append(&mut Self::wrap_as_step(enums_to_update, MigrationStep::UpdateEnum));
        result.append(&mut Self::wrap_as_step(indexes_to_rename, MigrationStep::UpdateIndex));
        result.append(&mut Self::wrap_as_step(indexes_to_delete, MigrationStep::DeleteIndex));
        result.append(&mut Self::wrap_as_step(indexes_to_create, MigrationStep::CreateIndex));
        result
    }

    /// Iterate over the models that are present in both schemas. The order is `(previous_model, next_model)`.
    fn model_pairs(&self) -> impl Iterator<Item = (&Model, &Model)> {
        self.previous.models().filter_map(move |previous_model| {
            self.next
                .find_model(&previous_model.name)
                .map(|next_model| (previous_model, next_model))
        })
    }

    fn models_to_create(&self) -> Vec<CreateModel> {
        let mut result = Vec::new();
        for next_model in self.next.models() {
            if !self.previous.has_model(&next_model.name()) {
                let step = CreateModel {
                    name: next_model.name().to_string(),
                    db_name: next_model.database_name.as_ref().cloned(),
                    embedded: next_model.is_embedded,
                };
                result.push(step);
            }
        }

        result
    }

    fn models_to_update(&self) -> Vec<UpdateModel> {
        let mut result = Vec::new();
        for previous_model in self.previous.models() {
            if let Some(next_model) = self.next.find_model(&previous_model.name) {
                let step = UpdateModel {
                    name: next_model.name.clone(),
                    new_name: None,
                    db_name: Self::diff(&previous_model.database_name, &next_model.database_name),
                    embedded: Self::diff(&previous_model.is_embedded, &next_model.is_embedded),
                };
                if step.is_any_option_set() {
                    result.push(step);
                }
            }
        }
        result
    }

    fn models_to_delete(&self) -> Vec<DeleteModel> {
        let mut result = Vec::new();
        for previous_model in self.previous.models() {
            if !self.next.has_model(&previous_model.name) {
                let step = DeleteModel {
                    name: previous_model.name().to_string(),
                };
                result.push(step);
            }
        }

        result
    }

    fn fields_to_create(&self) -> Vec<CreateField> {
        let mut result = Vec::new();
        for next_model in self.next.models() {
            for next_field in next_model.fields() {
                let must_create_field = match self.previous.find_model(&next_model.name) {
                    None => true,
                    Some(previous_model) => previous_model.find_field(&next_field.name).is_none(),
                };
                if must_create_field {
                    let step = CreateField {
                        model: next_model.name.clone(),
                        name: next_field.name.clone(),
                        tpe: next_field.field_type.clone(),
                        arity: next_field.arity,
                        db_name: next_field.database_name.clone(),
                        default: next_field.default_value.clone(),
                        id: next_field.id_info.clone(),
                        is_created_at: None,
                        is_updated_at: None,
                        is_unique: next_field.is_unique,
                        scalar_list: next_field.scalar_list_strategy,
                    };
                    result.push(step);
                }
            }
        }
        result
    }

    fn fields_to_delete(&self, models_to_delete: &Vec<DeleteModel>) -> Vec<DeleteField> {
        let mut result = Vec::new();
        for previous_model in self.previous.models() {
            let model_is_deleted = models_to_delete
                .iter()
                .find(|dm| dm.name == previous_model.name)
                .is_none();
            if model_is_deleted {
                for previous_field in previous_model.fields() {
                    let must_delete_field = match self.next.find_model(&previous_model.name) {
                        None => true,
                        Some(next_model) => next_model.find_field(&previous_field.name).is_none(),
                    };
                    if must_delete_field {
                        let step = DeleteField {
                            model: previous_model.name.clone(),
                            name: previous_field.name.clone(),
                        };
                        result.push(step);
                    }
                }
            }
        }
        result
    }

    fn fields_to_update(&self) -> Vec<UpdateField> {
        let mut result = Vec::new();
        for previous_model in self.previous.models() {
            for previous_field in previous_model.fields() {
                if let Some(next_field) = self
                    .next
                    .find_model(&previous_model.name)
                    .and_then(|m| m.find_field(&previous_field.name))
                {
                    let (p, n) = (previous_field, next_field);
                    let step = UpdateField {
                        model: previous_model.name.clone(),
                        name: p.name.clone(),
                        new_name: None,
                        tpe: Self::diff(&p.field_type, &n.field_type),
                        arity: Self::diff(&p.arity, &n.arity),
                        db_name: Self::diff(&p.database_name, &n.database_name),
                        is_created_at: None,
                        is_updated_at: None,
                        is_unique: Self::diff(&p.is_unique, &n.is_unique),
                        id_info: None,
                        default: Self::diff(&p.default_value, &n.default_value),
                        scalar_list: Self::diff(&p.scalar_list_strategy, &n.scalar_list_strategy),
                    };
                    if step.is_any_option_set() {
                        result.push(step);
                    }
                }
            }
        }
        result
    }

    fn enums_to_create(&self) -> Vec<CreateEnum> {
        let mut result = Vec::new();
        for next_enum in self.next.enums() {
            if !self.previous.has_enum(&next_enum.name()) {
                let step = CreateEnum {
                    name: next_enum.name().to_string(),
                    db_name: next_enum.database_name.clone(),
                    values: next_enum.values.clone(),
                };
                result.push(step);
            }
        }

        result
    }

    fn enums_to_delete(&self) -> Vec<DeleteEnum> {
        let mut result = Vec::new();
        for previous_enum in self.previous.enums() {
            if !self.next.has_enum(&previous_enum.name) {
                let step = DeleteEnum {
                    name: previous_enum.name().clone(),
                };
                result.push(step);
            }
        }

        result
    }

    fn enums_to_update(&self) -> Vec<UpdateEnum> {
        let mut result = Vec::new();
        for previous_enum in self.previous.enums() {
            if let Some(next_enum) = self.next.find_enum(&previous_enum.name) {
                let step = UpdateEnum {
                    name: next_enum.name.clone(),
                    new_name: None,
                    db_name: Self::diff(&previous_enum.database_name, &next_enum.database_name),
                    values: Self::diff(&previous_enum.values, &next_enum.values),
                };
                if step.is_any_option_set() {
                    result.push(step);
                }
            }
        }
        result
    }

    fn indexes_to_rename(&self) -> Vec<UpdateIndex> {
        self.model_pairs()
            .flat_map(|(previous_model, next_model)| {
                next_model
                    .indexes
                    .iter()
                    // Filter for indexes that existed but changed name.
                    .filter(move |next_index| {
                        previous_model.indexes.iter().any(|previous_index| {
                            previous_index.tpe == next_index.tpe
                                && previous_index.fields == next_index.fields
                                && previous_index.name != next_index.name
                        })
                    })
                    .map(move |next_index| UpdateIndex {
                        model: next_model.name.clone(),
                        fields: next_index.fields.clone(),
                        name: next_index.name.clone(),
                        tpe: next_index.tpe,
                    })
            })
            .collect()
    }

    fn indexes_to_create(&self, updated_indexes: &[UpdateIndex]) -> Vec<CreateIndex> {
        self.next
            .models()
            .map(|next_model| (next_model, self.previous.find_model(&next_model.name)))
            .flat_map(|(next_model, previous_model_opt)| {
                next_model
                    .indexes
                    .iter()
                    // Updated indexes should not be created.
                    .filter(move |next_index| {
                        !updated_indexes
                            .iter()
                            .any(|updated_index| updated_index.applies_to_index(next_index))
                    })
                    // Keep only the indexes that do not exist yet.
                    .filter(move |next_index| {
                        previous_model_opt
                            .map(|previous_model| !previous_model.has_index(next_index))
                            // Create the index if the model didn't exist before.
                            .unwrap_or(true)
                    })
                    .map(move |next_index| CreateIndex {
                        model: next_model.name.clone(),
                        name: next_index.name.clone(),
                        tpe: next_index.tpe,
                        fields: next_index.fields.clone(),
                    })
            })
            .collect()
    }

    fn indexes_to_delete(&self, updated_indexes: &[UpdateIndex]) -> Vec<DeleteIndex> {
        self.model_pairs()
            .flat_map(|(previous_model, next_model)| {
                previous_model
                    .indexes
                    .iter()
                    // Updated indexes should not be deleted
                    .filter(move |existing_index| {
                        !updated_indexes
                            .iter()
                            .any(|updated_index| updated_index.applies_to_index(existing_index))
                    })
                    // Keep only the indexes that do not exist anymore.
                    .filter(move |existing_index| !next_model.has_index(existing_index))
                    .map(move |existing_index| DeleteIndex {
                        fields: existing_index.fields.clone(),
                        tpe: existing_index.tpe,
                        model: previous_model.name.clone(),
                        name: existing_index.name.clone(),
                    })
            })
            .collect()
    }

    fn diff<T: PartialEq + Clone>(current: &T, updated: &T) -> Option<T> {
        if current == updated {
            None
        } else {
            Some(updated.clone())
        }
    }

    fn wrap_as_step<T, F>(steps: Vec<T>, mut wrap_fn: F) -> Vec<MigrationStep>
    where
        F: FnMut(T) -> MigrationStep,
    {
        steps.into_iter().map(|x| wrap_fn(x)).collect()
    }
}

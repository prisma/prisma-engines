#![allow(clippy::suspicious_operation_groupings)] // clippy is wrong there

mod names;
mod referential_actions;

use crate::{
    ast,
    common::constraint_names::ConstraintNames,
    configuration,
    diagnostics::{DatamodelError, Diagnostics},
    dml,
};
use datamodel_connector::ConnectorCapability;
use names::NamesValidator;

/// Helper for validating a datamodel.
///
/// When validating, we check if the datamodel is valid, and generate errors otherwise.
pub struct Validator<'a> {
    source: Option<&'a configuration::Datasource>,
}

/// State error message. Seeing this error means something went really wrong internally. It's the datamodel equivalent of a bluescreen.
const STATE_ERROR: &str = "Failed lookup of model, field or optional property during internal processing. This means that the internal representation was mutated incorrectly.";
const RELATION_ATTRIBUTE_NAME: &str = "relation";
const RELATION_ATTRIBUTE_NAME_WITH_AT: &str = "@relation";
const PRISMA_FORMAT_HINT: &str = "You can run `prisma format` to fix this automatically.";

impl<'a> Validator<'a> {
    /// Creates a new instance, with all builtin attributes registered.
    pub fn new(source: Option<&'a configuration::Datasource>) -> Validator<'a> {
        Self { source }
    }

    pub(crate) fn validate(&self, ast: &ast::SchemaAst, schema: &dml::Datamodel, diagnostics: &mut Diagnostics) {
        for model in schema.models() {
            let ast_model = ast.find_model(&model.name).expect(STATE_ERROR);

            //doing this here for now since I want to have all field names already generated
            // it might be possible to move this
            if let Some(pk) = &model.primary_key {
                if let Some(name) = &pk.name {
                    for field in model.fields() {
                        if let Some(err) = ConstraintNames::client_name_already_in_use(
                            name,
                            field.name(),
                            &model.name,
                            ast_model.span,
                            "@@id",
                        ) {
                            diagnostics.push_error(err);
                        }
                    }
                }
            }

            for index in &model.indices {
                //doing this here for now since I want to have all field names already generated
                // it might be possible to move this
                if let Some(name) = &index.name {
                    for field in model.fields() {
                        if let Some(err) = ConstraintNames::client_name_already_in_use(
                            name,
                            field.name(),
                            &model.name,
                            ast_model.span,
                            "@@unique",
                        ) {
                            diagnostics.push_error(err);
                        }
                    }
                }
            }

            if let Err(err) = self.validate_model_has_strict_unique_criteria(ast_model, model) {
                diagnostics.push_error(err);
            }

            if let Err(err) = self.validate_relations_not_ambiguous(ast, model) {
                diagnostics.push_error(err);
            }

            if let Err(ref mut the_errors) = self.validate_field_connector_specific(ast_model, model) {
                diagnostics.append(the_errors)
            }

            if let Err(ref mut the_errors) = self.validate_model_connector_specific(ast_model, model) {
                diagnostics.append(the_errors)
            }

            if let Err(ref mut the_errors) = self.validate_referenced_fields_for_relation(schema, ast_model, model) {
                diagnostics.append(the_errors);
            }
        }
    }

    pub(crate) fn post_standardisation_validate(
        &self,
        ast_schema: &ast::SchemaAst,
        schema: &dml::Datamodel,
        diagnostics: &mut Diagnostics,
    ) {
        let constraint_names = NamesValidator::new(schema, self.source);

        for model in schema.models() {
            let ast_model = ast_schema.find_model(&model.name).expect(STATE_ERROR);

            if let Some(pk) = &model.primary_key {
                if let Some(name) = &pk.db_name {
                    // Only for SQL Server for now...
                    if constraint_names.is_duplicate(name) {
                        let span = ast_model.id_attribute().span;
                        let message = "Given constraint name is already in use in the data model.";
                        let error = DatamodelError::new_attribute_validation_error(message, "id", span);

                        diagnostics.push_error(error);
                    }
                }
            }

            // TODO: Extend this check for other constraints. Now only used
            // for SQL Server default constraint names.
            for field in model.fields().filter(|f| f.is_scalar_field()) {
                if let Some(name) = field.default_value().and_then(|d| d.db_name()) {
                    let ast_field = ast_model.find_field_bang(field.name());

                    if constraint_names.is_duplicate(name) {
                        let message = "Given constraint name is already in use in the data model.";
                        let span = ast_field.span_for_argument("default", "map");
                        let error = DatamodelError::new_attribute_validation_error(message, "default", span);

                        diagnostics.push_error(error);
                    }
                }
            }

            let mut new_errors = self.validate_relation_arguments_bla(
                schema,
                ast_schema.find_model(&model.name).expect(STATE_ERROR),
                model,
                &constraint_names,
            );

            diagnostics.append(&mut new_errors);
        }
    }

    fn validate_model_has_strict_unique_criteria(
        &self,
        ast_model: &ast::Model,
        model: &dml::Model,
    ) -> Result<(), DatamodelError> {
        let loose_criterias = model.loose_unique_criterias();
        let suffix = if loose_criterias.is_empty() {
            "".to_string()
        } else {
            let criteria_descriptions: Vec<_> = loose_criterias
                .iter()
                .map(|criteria| {
                    let field_names: Vec<_> = criteria.fields.iter().map(|f| f.name.clone()).collect();
                    format!("- {}", field_names.join(", "))
                })
                .collect();
            format!(
                " The following unique criterias were not considered as they contain fields that are not required:\n{}",
                criteria_descriptions.join("\n")
            )
        };

        if model.strict_unique_criterias_disregarding_unsupported().is_empty() && !model.is_ignored {
            return Err(DatamodelError::new_model_validation_error(
                &format!(
                    "Each model must have at least one unique criteria that has only required fields. Either mark a single field with `@id`, `@unique` or add a multi field criterion with `@@id([])` or `@@unique([])` to the model.{suffix}",
                    suffix = suffix
                ),
                &model.name,
                ast_model.span,
            ));
        }

        Ok(())
    }

    fn validate_field_connector_specific(&self, ast_model: &ast::Model, model: &dml::Model) -> Result<(), Diagnostics> {
        let mut diagnostics = Diagnostics::new();

        if let Some(source) = self.source {
            let connector = &source.active_connector;
            for field in model.fields.iter() {
                if let Err(err) = connector.validate_field(field) {
                    diagnostics.push_error(DatamodelError::new_connector_error(
                        &err.to_string(),
                        ast_model.find_field_bang(field.name()).span,
                    ));
                }

                if let dml::Field::RelationField(ref rf) = field {
                    let actions = &[rf.relation_info.on_delete, rf.relation_info.on_update];

                    actions.iter().flatten().for_each(|action| {
                        if !connector.supports_referential_action(*action) {
                            let allowed_values: Vec<_> = connector
                                .referential_actions()
                                .iter()
                                .map(|f| format!("`{}`", f))
                                .collect();

                            let message = format!(
                                "Invalid referential action: `{}`. Allowed values: ({})",
                                action,
                                allowed_values.join(", "),
                            );

                            diagnostics.push_error(DatamodelError::new_attribute_validation_error(
                                &message,
                                "relation",
                                ast_model.find_field_bang(field.name()).span,
                            ));
                        }
                    });
                }
            }
        }

        diagnostics.to_result()
    }

    fn validate_model_connector_specific(&self, ast_model: &ast::Model, model: &dml::Model) -> Result<(), Diagnostics> {
        let mut diagnostics = Diagnostics::new();

        if let Some(source) = self.source {
            let connector = &source.active_connector;
            if let Err(err) = connector.validate_model(model) {
                diagnostics.push_error(DatamodelError::new_connector_error(&err.to_string(), ast_model.span))
            }
        }

        diagnostics.to_result()
    }

    fn validate_referenced_fields_for_relation(
        &self,
        datamodel: &dml::Datamodel,
        ast_model: &ast::Model,
        model: &dml::Model,
    ) -> Result<(), Diagnostics> {
        let mut errors = Diagnostics::new();

        for field in model.relation_fields() {
            let ast_field = ast_model.find_field_bang(&field.name);

            let rel_info = &field.relation_info;
            let related_model = datamodel.find_model(&rel_info.to).expect(STATE_ERROR);

            let fields_with_wrong_type: Vec<DatamodelError> = rel_info.fields.iter().zip(rel_info.references.iter())
                    .filter_map(|(base_field, referenced_field)| {
                        let base_field = model.find_field(base_field)?;
                        let referenced_field = related_model.find_field(referenced_field)?;

                        if base_field.field_type().is_compatible_with(&referenced_field.field_type()) {
                            return None
                        }

                        // Try harder to see if the final type is not the same.
                        // This check needs the connector, so it can't be in the dml
                        // crate.
                        if let Some(connector) = self.source.map(|source| &source.active_connector) {
                            let base_native_type = base_field.field_type().as_native_type().map(|(scalar, native)| (*scalar, native.serialized_native_type.clone())).or_else(|| -> Option<_> {
                                let field_type = base_field.field_type();
                                let scalar_type = field_type.as_scalar()?;

                                Some((*scalar_type, connector.default_native_type_for_scalar_type(scalar_type)))
                            });

                            let referenced_native_type = referenced_field.field_type().as_native_type().map(|(scalar, native)| (*scalar, native.serialized_native_type.clone())).or_else(|| -> Option<_> {
                                let field_type = referenced_field.field_type();
                                let scalar_type = field_type.as_scalar()?;

                                Some((*scalar_type, connector.default_native_type_for_scalar_type(scalar_type)))
                            });

                            if base_native_type.is_some() && base_native_type == referenced_native_type {
                                return None
                            }
                        }

                        Some(DatamodelError::new_attribute_validation_error(
                            &format!(
                                "The type of the field `{}` in the model `{}` is not matching the type of the referenced field `{}` in model `{}`.",
                                &base_field.name(),
                                &model.name,
                                &referenced_field.name(),
                                &related_model.name
                            ),
                            RELATION_ATTRIBUTE_NAME,
                            ast_field.span,
                        ))
                    })
                    .collect();

            if !rel_info.references.is_empty() && !errors.has_errors() {
                let strict_relation_field_order = self
                    .source
                    .map(|s| !s.active_connector.allows_relation_fields_in_arbitrary_order())
                    .unwrap_or(false);

                // when we have other errors already don't push this error additionally
                let references_unique_criteria = related_model.loose_unique_criterias().iter().any(|criteria| {
                    let mut criteria_field_names: Vec<_> = criteria.fields.iter().map(|f| f.name.to_owned()).collect();
                    criteria_field_names.sort();

                    let mut references_sorted = rel_info.references.clone();
                    references_sorted.sort();

                    criteria_field_names == references_sorted
                });

                let reference_order_correct = if strict_relation_field_order && rel_info.references.len() > 1 {
                    related_model.loose_unique_criterias().iter().any(|criteria| {
                        let criteria_fields = criteria.fields.iter().map(|f| f.name.as_str());
                        let references = rel_info.references.iter().map(|f| f.as_str());

                        let same_length = criteria_fields.len() == references.len();
                        let same_order = criteria_fields.zip(references).all(|(a, b)| a == b);

                        same_length && same_order
                    })
                } else {
                    true
                };

                let references_singular_id_field = rel_info.references.len() == 1
                    && related_model.field_is_primary(rel_info.references.first().unwrap());

                let is_many_to_many = {
                    // Back relation fields have not been added yet. So we must calculate this on our own.
                    match datamodel.find_related_field(field) {
                        Some((_, related_field)) => field.is_list() && related_field.is_list(),
                        None => false,
                    }
                };

                let must_reference_unique_criteria = match self.source {
                    Some(source) => !source.active_connector.supports_relations_over_non_unique_criteria(),
                    None => true,
                };

                if !references_unique_criteria && must_reference_unique_criteria {
                    errors.push_error(DatamodelError::new_validation_error(
                            &format!("The argument `references` must refer to a unique criteria in the related model `{}`. But it is referencing the following fields that are not a unique criteria: {}",
                                     &related_model.name,
                                     rel_info.references.join(", ")),
                            ast_field.span)
                        );
                } else if !reference_order_correct {
                    errors.push_error(DatamodelError::new_validation_error(
                        &format!("The argument `references` must refer to a unique criteria in the related model `{}` using the same order of fields. Please check the ordering in the following fields: `{}`.",
                                 &related_model.name,
                                 rel_info.references.join(", ")),
                        ast_field.span)
                    );
                }

                // TODO: This error is only valid for connectors that don't support native many to manys.
                // We only render this error if there's a singular id field. Otherwise we render a better error in a different function.
                if is_many_to_many
                    && !references_singular_id_field
                    && related_model.has_single_id_field()
                    && model.has_single_id_field()
                {
                    errors.push_error(DatamodelError::new_validation_error(
                            &format!(
                                "Many to many relations must always reference the id field of the related model. Change the argument `references` to use the id field of the related model `{}`. But it is referencing the following fields that are not the id: {}",
                                &related_model.name,
                                rel_info.references.join(", ")
                            ),
                            ast_field.span)
                        );
                }
            }

            if !rel_info.fields.is_empty()
                && !rel_info.references.is_empty()
                && rel_info.fields.len() != rel_info.references.len()
            {
                errors.push_error(DatamodelError::new_attribute_validation_error(
                    "You must specify the same number of fields in `fields` and `references`.",
                    RELATION_ATTRIBUTE_NAME,
                    ast_field.span,
                ));
            }

            if !errors.has_errors() {
                // don't output too much errors
                for err in fields_with_wrong_type {
                    errors.push_error(err);
                }
            }
        }

        errors.to_result()
    }

    fn validate_relation_arguments_bla<'dml>(
        &self,
        datamodel: &'dml dml::Datamodel,
        ast_model: &ast::Model,
        model: &dml::Model,
        constraint_names: &NamesValidator<'dml>,
    ) -> Diagnostics {
        let mut errors = Diagnostics::new();

        for field in model.relation_fields() {
            let ast_field = ast_model
                .fields
                .iter()
                .find(|ast_field| ast_field.name.name == field.name);

            let field_span = ast_field.map(|f| f.span).unwrap_or_else(ast::Span::empty);

            let rel_info = &field.relation_info;
            let related_model = datamodel.find_model(&rel_info.to).expect(STATE_ERROR);

            if let Some(name) = field.relation_info.fk_name.as_ref() {
                // Only for SQL Server for now...
                if constraint_names.is_duplicate(name) {
                    let span = ast_field
                        .map(|f| f.span_for_argument("relation", "map"))
                        .unwrap_or_else(ast::Span::empty);

                    let message = "Given constraint name is already in use in the data model.";
                    let error = DatamodelError::new_attribute_validation_error(message, RELATION_ATTRIBUTE_NAME, span);

                    errors.push_error(error);
                }
            }

            if let Some((_rel_field_idx, related_field)) = datamodel.find_related_field(field) {
                let related_field_rel_info = &related_field.relation_info;

                if related_model.is_ignored && !field.is_ignored && !model.is_ignored {
                    let message = format!(
                        "The relation field `{}` on Model `{}` must specify the `@ignore` attribute, because the model {} it is pointing to is marked ignored.",
                        &field.name, &model.name, &related_model.name
                    );

                    errors.push_error(DatamodelError::new_attribute_validation_error(
                        &message, "ignore", field_span,
                    ));
                }

                // ONE TO MANY
                if field.is_singular() && related_field.is_list() {
                    if rel_info.fields.is_empty() {
                        let message = format!(
                            "The relation field `{}` on Model `{}` must specify the `fields` argument in the {} attribute. {}",
                            &field.name, &model.name, RELATION_ATTRIBUTE_NAME_WITH_AT, PRISMA_FORMAT_HINT
                        );

                        errors.push_error(DatamodelError::new_attribute_validation_error(
                            &message,
                            RELATION_ATTRIBUTE_NAME,
                            field_span,
                        ));
                    }

                    if rel_info.references.is_empty() {
                        errors.push_error(DatamodelError::new_attribute_validation_error(
                        &format!(
                            "The relation field `{}` on Model `{}` must specify the `references` argument in the {} attribute.",
                            &field.name, &model.name, RELATION_ATTRIBUTE_NAME_WITH_AT
                            ),
                        RELATION_ATTRIBUTE_NAME,
                        field_span,
                        ));
                    }
                }

                if field.is_list()
                    && !related_field.is_list()
                    && (!rel_info.fields.is_empty() || !rel_info.references.is_empty())
                {
                    let message = format!(
                        "The relation field `{}` on Model `{}` must not specify the `fields` or `references` argument in the {} attribute. You must only specify it on the opposite field `{}` on model `{}`.",
                        &field.name,
                        &model.name,
                        RELATION_ATTRIBUTE_NAME_WITH_AT,
                        &related_field.name,
                        &related_model.name
                    );

                    errors.push_error(DatamodelError::new_attribute_validation_error(
                        &message,
                        RELATION_ATTRIBUTE_NAME,
                        field_span,
                    ));
                }

                if field.is_list()
                    && !related_field.is_list()
                    && (rel_info.on_delete.is_some() || rel_info.on_update.is_some())
                {
                    let message = &format!(
                        "The relation field `{}` on Model `{}` must not specify the `onDelete` or `onUpdate` argument in the {} attribute. You must only specify it on the opposite field `{}` on model `{}`, or in case of a many to many relation, in an explicit join table.",
                        &field.name, &model.name, RELATION_ATTRIBUTE_NAME_WITH_AT, &related_field.name, &related_model.name
                    );

                    errors.push_error(DatamodelError::new_attribute_validation_error(
                        message,
                        RELATION_ATTRIBUTE_NAME,
                        field_span,
                    ));
                }

                // ONE TO ONE
                if field.is_singular() && related_field.is_singular() {
                    if rel_info.fields.is_empty() && related_field_rel_info.fields.is_empty() {
                        let message = format!(
                            "The relation fields `{}` on Model `{}` and `{}` on Model `{}` do not provide the `fields` argument in the {} attribute. You have to provide it on one of the two fields.",
                            &field.name, &model.name, &related_field.name, &related_model.name, RELATION_ATTRIBUTE_NAME_WITH_AT
                        );

                        errors.push_error(DatamodelError::new_attribute_validation_error(
                            &message,
                            RELATION_ATTRIBUTE_NAME,
                            field_span,
                        ));
                    }

                    if rel_info.references.is_empty() && related_field_rel_info.references.is_empty() {
                        let message = format!(
                            "The relation fields `{}` on Model `{}` and `{}` on Model `{}` do not provide the `references` argument in the {} attribute. You have to provide it on one of the two fields.",
                            &field.name, &model.name, &related_field.name, &related_model.name, RELATION_ATTRIBUTE_NAME_WITH_AT
                        );

                        errors.push_error(DatamodelError::new_attribute_validation_error(
                            &message,
                            RELATION_ATTRIBUTE_NAME,
                            field_span,
                        ));
                    }

                    if !rel_info.references.is_empty() && !related_field_rel_info.references.is_empty() {
                        let message = format!(
                            "The relation fields `{}` on Model `{}` and `{}` on Model `{}` both provide the `references` argument in the {} attribute. You have to provide it only on one of the two fields.",
                            &field.name, &model.name, &related_field.name, &related_model.name, RELATION_ATTRIBUTE_NAME_WITH_AT
                        );

                        errors.push_error(DatamodelError::new_attribute_validation_error(
                            &message,
                            RELATION_ATTRIBUTE_NAME,
                            field_span,
                        ));
                    }

                    if (rel_info.on_delete.is_some() || rel_info.on_update.is_some())
                        && (related_field_rel_info.on_delete.is_some() || related_field_rel_info.on_update.is_some())
                    {
                        let message = format!(
                            "The relation fields `{}` on Model `{}` and `{}` on Model `{}` both provide the `onDelete` or `onUpdate` argument in the {} attribute. You have to provide it only on one of the two fields.",
                            &field.name, &model.name, &related_field.name, &related_model.name, RELATION_ATTRIBUTE_NAME_WITH_AT
                        );

                        errors.push_error(DatamodelError::new_attribute_validation_error(
                            &message,
                            RELATION_ATTRIBUTE_NAME,
                            field_span,
                        ));
                    } else if rel_info.fields.is_empty()
                        && (rel_info.on_delete.is_some() || rel_info.on_update.is_some())
                    {
                        let message = &format!(
                            "The relation field `{}` on Model `{}` must not specify the `onDelete` or `onUpdate` argument in the {} attribute. You must only specify it on the opposite field `{}` on model `{}`.",
                            &field.name, &model.name, RELATION_ATTRIBUTE_NAME_WITH_AT, &related_field.name, &related_model.name
                        );

                        errors.push_error(DatamodelError::new_attribute_validation_error(
                            message,
                            RELATION_ATTRIBUTE_NAME,
                            field_span,
                        ));
                    }

                    if !rel_info.fields.is_empty() && !related_field_rel_info.fields.is_empty() {
                        let message = format!(
                            "The relation fields `{}` on Model `{}` and `{}` on Model `{}` both provide the `fields` argument in the {} attribute. You have to provide it only on one of the two fields.",
                            &field.name, &model.name, &related_field.name, &related_model.name, RELATION_ATTRIBUTE_NAME_WITH_AT
                        );

                        errors.push_error(DatamodelError::new_attribute_validation_error(
                            &message,
                            RELATION_ATTRIBUTE_NAME,
                            field_span,
                        ));
                    }

                    if !errors.has_errors() {
                        if !rel_info.fields.is_empty() && !related_field_rel_info.references.is_empty() {
                            let message = format!(
                                "The relation field `{}` on Model `{}` provides the `fields` argument in the {} attribute. And the related field `{}` on Model `{}` provides the `references` argument. You must provide both arguments on the same side.",
                                &field.name, &model.name, RELATION_ATTRIBUTE_NAME_WITH_AT, &related_field.name, &related_model.name,
                            );

                            errors.push_error(DatamodelError::new_attribute_validation_error(
                                &message,
                                RELATION_ATTRIBUTE_NAME,
                                field_span,
                            ));
                        }

                        if !rel_info.references.is_empty() && !related_field_rel_info.fields.is_empty() {
                            let message = format!(
                                "The relation field `{}` on Model `{}` provides the `references` argument in the {} attribute. And the related field `{}` on Model `{}` provides the `fields` argument. You must provide both arguments on the same side.",
                                &field.name, &model.name, RELATION_ATTRIBUTE_NAME_WITH_AT, &related_field.name, &related_model.name,
                            );

                            errors.push_error(DatamodelError::new_attribute_validation_error(
                                &message,
                                RELATION_ATTRIBUTE_NAME,
                                field_span,
                            ));
                        }
                    }

                    if !errors.has_errors() && field.is_required() && !related_field_rel_info.references.is_empty() {
                        let message = format!(
                            "The relation field `{}` on Model `{}` is required. This is no longer valid because it's not possible to enforce this constraint on the database level. Please change the field type from `{}` to `{}?` to fix this.",
                            &field.name, &model.name, &related_model.name, &related_model.name,
                        );

                        errors.push_error(DatamodelError::new_attribute_validation_error(
                            &message,
                            RELATION_ATTRIBUTE_NAME,
                            field_span,
                        ));
                    }
                }

                // MANY TO MANY
                if field.is_list() && related_field.is_list() && !related_model.has_single_id_field() {
                    let message = format!(
                        "The relation field `{}` on Model `{}` references `{}` which does not have an `@id` field. Models without `@id` cannot be part of a many to many relation. Use an explicit intermediate Model to represent this relationship.",
                        &field.name,
                        &model.name,
                        &related_model.name,
                    );

                    errors.push_error(DatamodelError::new_field_validation_error(
                        &message,
                        &model.name,
                        &field.name,
                        field_span,
                    ));
                }

                if !field.is_list()
                    && self
                        .source
                        .map(|source| &source.active_connector)
                        .map(|connector| connector.has_capability(ConnectorCapability::ReferenceCycleDetection))
                        .unwrap_or_default()
                {
                    referential_actions::detect_cycles(datamodel, model, field, field_span, &mut errors);
                }
            } else {
                let message = format!(
                    "The relation field `{}` on Model `{}` is missing an opposite relation field on the model `{}`. Either run `prisma format` or add it manually.",
                    &field.name,
                    &model.name,
                    &related_model.name,
                );

                errors.push_error(DatamodelError::new_field_validation_error(
                    &message,
                    &model.name,
                    &field.name,
                    field_span,
                ));
            }
        }

        errors
    }

    /// Elegantly checks if any relations in the model are ambigious.
    fn validate_relations_not_ambiguous(
        &self,
        ast_schema: &ast::SchemaAst,
        model: &dml::Model,
    ) -> Result<(), DatamodelError> {
        for field_a in model.relation_fields() {
            for field_b in model.relation_fields() {
                if field_a != field_b {
                    let rel_a = &field_a.relation_info;
                    let rel_b = &field_b.relation_info;
                    if rel_a.to != model.name && rel_b.to != model.name {
                        // Not a self relation
                        // but pointing to the same foreign model,
                        // and also no names set.
                        if rel_a.to == rel_b.to && rel_a.name == rel_b.name {
                            if rel_a.name.is_empty() {
                                let message = format!(
                                    "Ambiguous relation detected. The fields `{}` and `{}` in model `{}` both refer to `{}`. Please provide different relation names for them by adding `@relation(<name>).",
                                    &field_a.name,
                                    &field_b.name,
                                    &model.name,
                                    &rel_a.to
                                );

                                // unnamed relation
                                return Err(DatamodelError::new_model_validation_error(
                                    &message,
                                    &model.name,
                                    ast_schema
                                        .find_field(&model.name, &field_a.name)
                                        .expect(STATE_ERROR)
                                        .span,
                                ));
                            } else {
                                let message = format!(
                                    "Wrongly named relation detected. The fields `{}` and `{}` in model `{}` both use the same relation name. Please provide different relation names for them through `@relation(<name>).",
                                    &field_a.name,
                                    &field_b.name,
                                    &model.name,
                                );

                                // explicitly named relation
                                return Err(DatamodelError::new_model_validation_error(
                                    &message,
                                    &model.name,
                                    ast_schema
                                        .find_field(&model.name, &field_a.name)
                                        .expect(STATE_ERROR)
                                        .span,
                                ));
                            }
                        }
                    } else if rel_a.to == model.name && rel_b.to == model.name {
                        // This is a self-relation with at least two fields.

                        // Named self relations are ambiguous when they involve more than two fields.
                        for field_c in model.relation_fields() {
                            if field_a != field_c && field_b != field_c {
                                let rel_c = &field_c.relation_info;
                                if rel_c.to == model.name && rel_a.name == rel_b.name && rel_a.name == rel_c.name {
                                    if rel_a.name.is_empty() {
                                        // unnamed relation
                                        return Err(DatamodelError::new_model_validation_error(
                                                        &format!(
                                                            "Unnamed self relation detected. The fields `{}`, `{}` and `{}` in model `{}` have no relation name. Please provide a relation name for one of them by adding `@relation(<name>).",
                                                            &field_a.name,
                                                            &field_b.name,
                                                            &field_c.name,
                                                            &model.name
                                                        ),
                                                        &model.name,
                                                        ast_schema
                                                            .find_field(&model.name, &field_a.name)
                                                            .expect(STATE_ERROR)
                                                            .span,
                                                    ));
                                    } else {
                                        return Err(DatamodelError::new_model_validation_error(
                                            &format!(
                                                "Wrongly named self relation detected. The fields `{}`, `{}` and `{}` in model `{}` have the same relation name. At most two relation fields can belong to the same relation and therefore have the same name. Please assign a different relation name to one of them.",
                                                &field_a.name,
                                                &field_b.name,
                                                &field_c.name,
                                                &model.name
                                            ),
                                            &model.name,
                                            ast_schema
                                                .find_field(&model.name, &field_a.name)
                                                .expect(STATE_ERROR)
                                                .span,
                                        ));
                                    }
                                }
                            }
                        }

                        // Ambiguous unnamed self relation: two fields are enough.
                        if rel_a.name.is_empty() && rel_b.name.is_empty() {
                            // A self relation, but there are at least two fields without a name.
                            return Err(DatamodelError::new_model_validation_error(
                                &format!(
                                    "Ambiguous self relation detected. The fields `{}` and `{}` in model `{}` both refer to `{}`. If they are part of the same relation add the same relation name for them with `@relation(<name>)`.",
                                    &field_a.name,
                                    &field_b.name,
                                    &model.name,
                                    &rel_a.to
                                ),
                                &model.name,
                                ast_schema
                                    .find_field(&model.name, &field_a.name)
                                    .expect(STATE_ERROR)
                                    .span,
                            ));
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

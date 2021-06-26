use std::collections::HashSet;

use super::{super::helpers::*, AttributeValidator};
use crate::{
    ast,
    common::{preview_features::PreviewFeature, RelationNames},
    diagnostics::DatamodelError,
    dml, Field,
};

/// Prismas builtin `@relation` attribute.
pub struct RelationAttributeValidator {
    preview_features: HashSet<PreviewFeature>,
}

impl RelationAttributeValidator {
    pub fn new(preview_features: HashSet<PreviewFeature>) -> Self {
        Self { preview_features }
    }
}

impl AttributeValidator<dml::Field> for RelationAttributeValidator {
    fn attribute_name(&self) -> &'static str {
        "relation"
    }

    fn validate_and_apply(&self, args: &mut Arguments<'_>, field: &mut dml::Field) -> Result<(), DatamodelError> {
        if let dml::Field::RelationField(rf) = field {
            if let Ok(name_arg) = args.default_arg("name") {
                let name = name_arg.as_str()?;

                if name.is_empty() {
                    return self
                        .new_attribute_validation_error("A relation cannot have an empty name.", name_arg.span());
                }

                rf.relation_info.name = name.to_owned();
            }

            if let Ok(related_fields) = args.arg("references") {
                rf.relation_info.references = related_fields.as_array().to_literal_vec()?;
            }

            if let Ok(base_fields) = args.arg("fields") {
                rf.relation_info.fields = base_fields.as_array().to_literal_vec()?;
            }

            if let Ok(on_delete) = args.arg("onDelete") {
                rf.relation_info.on_delete = Some(on_delete.as_referential_action()?);
            }

            if let Ok(on_update) = args.arg("onUpdate") {
                rf.relation_info.on_update = Some(on_update.as_referential_action()?);
            }

            Ok(())
        } else {
            self.new_attribute_validation_error("Invalid field type, not a relation.", args.span())
        }
    }

    fn serialize(&self, field: &dml::Field, datamodel: &dml::Datamodel) -> Vec<ast::Attribute> {
        if let dml::Field::RelationField(rf) = field {
            let mut args = Vec::new();
            let relation_info = &rf.relation_info;
            let parent_model = datamodel.find_model_by_relation_field_ref(rf).unwrap();

            let related_model = datamodel
                .find_model(&relation_info.to)
                .unwrap_or_else(|| panic!("Related model not found: {}.", relation_info.to));

            let mut all_related_ids = related_model.id_field_names();
            let has_default_name = relation_info.name
                == RelationNames::name_for_unambiguous_relation(&relation_info.to, &parent_model.name);

            if !relation_info.name.is_empty() && (!has_default_name || parent_model.name == related_model.name) {
                args.push(ast::Argument::new_string("", &relation_info.name));
            }

            let mut relation_fields = relation_info.references.clone();

            relation_fields.sort();
            all_related_ids.sort();

            if !relation_info.fields.is_empty() {
                let mut fields: Vec<ast::Expression> = Vec::new();
                for field in &relation_info.fields {
                    fields.push(ast::Expression::ConstantValue(field.clone(), ast::Span::empty()));
                }

                args.push(ast::Argument::new_array("fields", fields));
            }

            // if we are on the physical field
            if !relation_info.references.is_empty() {
                let is_many_to_many = match &field {
                    Field::RelationField(relation_field) => {
                        let (_, related_field) = datamodel.find_related_field(relation_field).unwrap();
                        relation_field.arity.is_list() && related_field.arity.is_list()
                    }
                    _ => false,
                };

                let mut related_fields: Vec<ast::Expression> = Vec::with_capacity(relation_info.references.len());
                for related_field in &relation_info.references {
                    related_fields.push(ast::Expression::ConstantValue(
                        related_field.clone(),
                        ast::Span::empty(),
                    ));
                }

                if !is_many_to_many {
                    args.push(ast::Argument::new_array("references", related_fields));
                }
            }

            if self.preview_features.contains(&PreviewFeature::ReferentialActions) {
                if let Some(ref_action) = relation_info.on_delete {
                    if rf.default_on_delete_action() != ref_action {
                        let expression = ast::Expression::ConstantValue(ref_action.to_string(), ast::Span::empty());
                        args.push(ast::Argument::new("onDelete", expression));
                    }
                }

                if let Some(ref_action) = relation_info.on_update {
                    if rf.default_on_update_action() != ref_action {
                        let expression = ast::Expression::ConstantValue(ref_action.to_string(), ast::Span::empty());
                        args.push(ast::Argument::new("onUpdate", expression));
                    }
                }
            }

            if !args.is_empty() {
                return vec![ast::Attribute::new(self.attribute_name(), args)];
            }
        }

        vec![]
    }
}

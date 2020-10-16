use super::{super::helpers::*, AttributeValidator};
use crate::common::RelationNames;
use crate::diagnostics::DatamodelError;
use crate::{ast, dml, Field};

/// Prismas builtin `@relation` attribute.
pub struct RelationAttributeValidator {}

impl AttributeValidator<dml::Field> for RelationAttributeValidator {
    fn attribute_name(&self) -> &'static str {
        &"relation"
    }
    fn validate_and_apply(&self, args: &mut Arguments, field: &mut dml::Field) -> Result<(), DatamodelError> {
        if let dml::Field::RelationField(rf) = field {
            if let Ok(name_arg) = args.default_arg("name") {
                let name = name_arg.as_str()?;

                if name.is_empty() {
                    return self
                        .new_attribute_validation_error("A relation cannot have an empty name.", name_arg.span());
                }

                rf.relation_info.name = name;
            }

            if let Ok(related_fields) = args.arg("references") {
                rf.relation_info.to_fields = related_fields.as_array().to_literal_vec()?;
            }

            if let Ok(base_fields) = args.arg("fields") {
                rf.relation_info.fields = base_fields.as_array().to_literal_vec()?;
            }

            // TODO: bring `onDelete` back once `prisma migrate` is a thing
            //            if let Ok(on_delete) = args.arg("onDelete") {
            //                relation_info.on_delete = on_delete.parse_literal::<dml::OnDeleteStrategy>()?;
            //            }

            Ok(())
        } else {
            self.new_attribute_validation_error("Invalid field type, not a relation.", args.span())
        }
    }

    fn serialize(&self, field: &dml::Field, datamodel: &dml::Datamodel) -> Result<Vec<ast::Attribute>, DatamodelError> {
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

            let mut relation_fields = relation_info.to_fields.clone();

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
            if !relation_info.to_fields.is_empty() {
                let is_many_to_many = match &field {
                    Field::RelationField(relation_field) => {
                        let related_field = datamodel.find_related_field(&relation_field).unwrap();
                        relation_field.arity.is_list() && related_field.arity.is_list()
                    }
                    _ => false,
                };

                let mut related_fields: Vec<ast::Expression> = Vec::with_capacity(relation_info.to_fields.len());
                for related_field in &relation_info.to_fields {
                    related_fields.push(ast::Expression::ConstantValue(
                        related_field.clone(),
                        ast::Span::empty(),
                    ));
                }

                if !is_many_to_many {
                    args.push(ast::Argument::new_array("references", related_fields));
                }
            }

            if relation_info.on_delete != dml::OnDeleteStrategy::None {
                args.push(ast::Argument::new_constant(
                    "onDelete",
                    &relation_info.on_delete.to_string(),
                ));
            }

            if !args.is_empty() {
                return Ok(vec![ast::Attribute::new(self.attribute_name(), args)]);
            }
        }
        Ok(vec![])
    }
}

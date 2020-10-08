use super::{super::helpers::*, AttributeValidator};
use crate::ast;
use crate::dml;
use crate::error::{DatamodelError, ErrorCollection};

// BTreeMap has a strictly defined order.
// That's important since rendering depends on that order.
use std::collections::{BTreeMap, HashMap};

/// Struct which holds a list of attribute validators and automatically
/// picks the right one for each attribute in the given object.
pub struct AttributeListValidator<T> {
    known_attributes: BTreeMap<String, Box<dyn AttributeValidator<T>>>,
}

impl<T: 'static> AttributeListValidator<T> {
    pub fn new() -> Self {
        AttributeListValidator {
            known_attributes: BTreeMap::new(),
        }
    }

    /// Adds an attribute validator.
    pub fn add(&mut self, validator: Box<dyn AttributeValidator<T>>) {
        let name = validator.attribute_name();

        if self.known_attributes.contains_key(name) {
            panic!("Duplicate attribute definition: {:?}", name);
        }

        self.known_attributes.insert(String::from(name), validator);
    }

    /// For each attribute in the given object, picks the correct
    /// attribute definition and uses it to validate and apply the attribute.
    pub fn validate_and_apply(&self, ast: &dyn ast::WithAttributes, t: &mut T) -> Result<(), ErrorCollection> {
        let mut errors = ErrorCollection::new();

        let mut attribute_counts = HashMap::new();
        for attribute in ast.attributes() {
            match attribute_counts.get_mut(&attribute.name.name) {
                None => {
                    attribute_counts.insert(&attribute.name.name, 1);
                    
                }
                Some(count) => *count += 1,
            }
        }

        errors.ok()?;

        for attribute in ast.attributes() {
            match self.known_attributes.get(&attribute.name.name) {
                Some(validator) => {
                    let mut arguments = Arguments::new(&attribute.arguments, attribute.span);

                    let attribute_count = attribute_counts.get(&attribute.name.name).unwrap();
                    if *attribute_count > 1 && !validator.is_duplicate_definition_allowed() {
                        errors.push(DatamodelError::new_duplicate_attribute_error(
                            &attribute.name.name,
                            attribute.name.span,
                        ));
                    }

                    if let Err(mut errs) = arguments.check_for_duplicate_named_arguments() {
                        errors.append(&mut errs);
                    }

                    if let Err(mut errs) = arguments.check_for_multiple_unnamed_arguments(&attribute.name.name) {
                        errors.append(&mut errs);
                    }

                    let attribute_validation_result = validator.validate_and_apply(&mut arguments, t);

                    match attribute_validation_result {
                        Err(DatamodelError::ArgumentNotFound { argument_name, span }) => {
                            errors.push(DatamodelError::new_attribute_argument_not_found_error(
                                &argument_name,
                                &attribute.name.name,
                                span,
                            ))
                        }
                        Err(err) => {
                            errors.push(err);
                        }
                        _ => {
                            // We only check for unused arguments if attribute parsing succeeded.
                            if let Err(mut errs) = arguments.check_for_unused_arguments() {
                                errors.append(&mut errs);
                            }
                        }
                    }
                }
                None => {
                    if !attribute.name.name.is_empty() && !attribute.name.name.contains('.') {
                        errors.push(DatamodelError::new_attribute_not_known_error(
                            &attribute.name.name,
                            attribute.name.span,
                        ))
                    }
                }
            };
        }

        errors.ok()?;

        Ok(())
    }

    pub fn serialize(&self, t: &T, datamodel: &dml::Datamodel) -> Result<Vec<ast::Attribute>, ErrorCollection> {
        let mut errors = ErrorCollection::new();
        let mut result: Vec<ast::Attribute> = Vec::new();

        for attribute in self.known_attributes.values() {
            match attribute.serialize(t, datamodel) {
                Ok(mut attributes) => result.append(&mut attributes),
                Err(err) => errors.push(err),
            };
        }

        errors.ok()?;

        Ok(result)
    }
}

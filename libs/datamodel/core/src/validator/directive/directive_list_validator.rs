use super::{DirectiveScope, DirectiveValidator};
use crate::ast;
use crate::dml;
use crate::error::{DatamodelError, ErrorCollection};

// BTreeMap has a strictly defined order.
// That's important since rendering depends on that order.
use failure::_core::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};

/// Struct which holds a list of directive validators and automatically
/// picks the right one for each directive in the given object.
pub struct DirectiveListValidator<T> {
    known_directives: BTreeMap<String, Box<dyn DirectiveValidator<T>>>,
}

impl<T: 'static> DirectiveListValidator<T> {
    /// Creates a new instance.
    #[allow(unused)]
    pub fn new() -> Self {
        DirectiveListValidator {
            known_directives: BTreeMap::new(),
        }
    }

    /// Adds a directive validator.
    pub fn add(&mut self, validator: Box<dyn DirectiveValidator<T>>) {
        let name = validator.directive_name();

        if self.known_directives.contains_key(name) {
            panic!("Duplicate directive definition: {:?}", name);
        }

        self.known_directives.insert(String::from(name), validator);
    }

    /// Adds a directive validator with a namespace scope.
    fn add_scoped(&mut self, validator: Box<dyn DirectiveValidator<T>>, scope: &str) {
        let boxed: Box<dyn DirectiveValidator<T>> = Box::new(DirectiveScope::new(validator, scope));
        self.add(boxed)
    }

    /// Adds all directive validators from the given list.
    #[allow(unused)]
    fn add_all(&mut self, validators: Vec<Box<dyn DirectiveValidator<T>>>) {
        for validator in validators {
            self.add(validator);
        }
    }

    /// Adds all directive validators from the given list, with a namespace scope.
    pub fn add_all_scoped(&mut self, validators: Vec<Box<dyn DirectiveValidator<T>>>, scope: &str) {
        for validator in validators {
            self.add_scoped(validator, scope);
        }
    }

    /// For each directive in the given object, picks the correct
    /// directive definition and uses it to validate and apply the directive.
    pub fn validate_and_apply(&self, ast: &dyn ast::WithDirectives, t: &mut T) -> Result<(), ErrorCollection> {
        let mut errors = ErrorCollection::new();

        let mut directive_counts = HashMap::new();
        for directive in ast.directives() {
            match directive_counts.get_mut(&directive.name.name) {
                None => {
                    directive_counts.insert(&directive.name.name, 1);
                    ()
                }
                Some(count) => *count += 1,
            }
        }

        errors.ok()?;

        // validation orders:
        // @default before @id -> @id needs access to the default value on the field.
        // @relation before @map -> @map needs access to the relation info
        let mut cloned_directives = ast.directives().clone();
        cloned_directives.sort_by(|a, b| match (a.name.name.as_ref(), b.name.name.as_ref()) {
            ("default", "id") => Ordering::Less,
            ("id", "default") => Ordering::Greater,
            ("relation", "map") => Ordering::Less,
            ("map", "relation") => Ordering::Greater,
            _ => a.name.name.partial_cmp(&b.name.name).unwrap(),
        });

        for directive in cloned_directives {
            match self.known_directives.get(&directive.name.name) {
                Some(validator) => {
                    let mut arguments = super::Args::new(&directive.arguments, directive.span);

                    let directive_count = directive_counts.get(&directive.name.name).unwrap();
                    if *directive_count > 1 && !validator.is_duplicate_definition_allowed() {
                        errors.push(DatamodelError::new_duplicate_directive_error(
                            &directive.name.name,
                            directive.name.span,
                        ));
                    }

                    if let Err(mut errs) = arguments.check_for_duplicate_arguments() {
                        errors.append(&mut errs);
                    }

                    let directive_validation_result = validator.validate_and_apply(&mut arguments, t);

                    match directive_validation_result {
                        Err(DatamodelError::ArgumentNotFound { argument_name, span }) => {
                            errors.push(DatamodelError::new_directive_argument_not_found_error(
                                &argument_name,
                                &directive.name.name,
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
                None => errors.push(DatamodelError::new_directive_not_known_error(
                    &directive.name.name,
                    directive.name.span,
                )),
            };
        }

        errors.ok()?;

        Ok(())
    }

    pub fn serialize(&self, t: &T, datamodel: &dml::Datamodel) -> Result<Vec<ast::Directive>, ErrorCollection> {
        let mut errors = ErrorCollection::new();
        let mut result: Vec<ast::Directive> = Vec::new();

        for directive in self.known_directives.values() {
            match directive.serialize(t, datamodel) {
                Ok(mut directives) => result.append(&mut directives),
                Err(err) => errors.push(err),
            };
        }

        errors.ok()?;

        Ok(result)
    }
}

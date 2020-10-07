use crate::{
  ast::{self, WithIdentifier, WithName},
  dml,
  error::{DatamodelError, MessageCollection},
};

pub struct Precheck {}

impl Precheck {
    pub fn precheck(datamodel: &ast::SchemaAst) -> Result<(), MessageCollection> {
        let mut messages = MessageCollection::new();

        let mut top_level_types_checker = DuplicateChecker::new();
        let mut sources_checker = DuplicateChecker::new();
        let mut generators_checker = DuplicateChecker::new();

        for top in &datamodel.tops {
            let error_fn = |existing: &ast::Top| {
                DatamodelError::new_duplicate_top_error(
                    top.name(),
                    top.get_type(),
                    existing.get_type(),
                    top.identifier().span,
                )
            };
            match top {
                ast::Top::Enum(enum_type) => {
                    Self::assert_is_not_a_reserved_scalar_type(&enum_type.name, &mut messages);
                    top_level_types_checker.check_if_duplicate_exists(top, error_fn);
                    Self::precheck_enum(&enum_type, &mut messages);
                }
                ast::Top::Model(model) => {
                    Self::assert_is_not_a_reserved_scalar_type(&model.name, &mut messages);
                    top_level_types_checker.check_if_duplicate_exists(top, error_fn);
                    Self::precheck_model(&model, &mut messages);
                }
                ast::Top::Type(custom_type) => {
                    Self::assert_is_not_a_reserved_scalar_type(&custom_type.name, &mut messages);
                    top_level_types_checker.check_if_duplicate_exists(top, error_fn);
                }
                ast::Top::Source(source) => {
                    Self::assert_is_not_a_reserved_scalar_type(&source.name, &mut messages);
                    sources_checker.check_if_duplicate_exists(top, error_fn);
                    Self::precheck_source_config(&source, &mut messages);
                }
                ast::Top::Generator(generator) => {
                    Self::assert_is_not_a_reserved_scalar_type(&generator.name, &mut messages);
                    generators_checker.check_if_duplicate_exists(top, error_fn);
                    Self::precheck_generator_config(&generator, &mut messages);
                }
            }
        }

        messages.append(&mut top_level_types_checker.errors());
        messages.append(&mut sources_checker.errors());
        messages.append(&mut generators_checker.errors());

        messages.ok()
    }

    fn assert_is_not_a_reserved_scalar_type(identifier: &ast::Identifier, messages: &mut MessageCollection) {
        if dml::ScalarType::from_str(&identifier.name).is_ok() {
            messages.push_error(DatamodelError::new_reserved_scalar_type_error(
                &identifier.name,
                identifier.span,
            ));
        }
    }

    fn precheck_enum(enum_type: &ast::Enum, messages: &mut MessageCollection) {
        let mut checker = DuplicateChecker::new();
        for value in &enum_type.values {
            checker.check_if_duplicate_exists(value, |_| {
                DatamodelError::new_duplicate_enum_value_error(&enum_type.name.name, &value.name.name, value.span)
            });
        }
        messages.append(&mut checker.errors());
    }

    fn precheck_model(model: &ast::Model, messages: &mut MessageCollection) {
        let mut checker = DuplicateChecker::new();
        for field in &model.fields {
            checker.check_if_duplicate_exists(field, |_| {
                DatamodelError::new_duplicate_field_error(&model.name.name, &field.name.name, field.identifier().span)
            });
        }
        messages.append(&mut checker.errors());
    }

    fn precheck_generator_config(config: &ast::GeneratorConfig, messages: &mut MessageCollection) {
        let mut checker = DuplicateChecker::new();
        for arg in &config.properties {
            checker.check_if_duplicate_exists(arg, |_| {
                DatamodelError::new_duplicate_config_key_error(
                    &format!("generator configuration \"{}\"", config.name.name),
                    &arg.name.name,
                    arg.identifier().span,
                )
            });
        }
        messages.append(&mut checker.errors());
    }

    fn precheck_source_config(config: &ast::SourceConfig, messages: &mut MessageCollection) {
        let mut checker = DuplicateChecker::new();
        for arg in &config.properties {
            checker.check_if_duplicate_exists(arg, |_| {
                DatamodelError::new_duplicate_config_key_error(
                    &format!("datasource configuration \"{}\"", config.name.name),
                    &arg.name.name,
                    arg.identifier().span,
                )
            });
        }
        messages.append(&mut checker.errors());
    }
}

struct DuplicateChecker<'a, T: WithName> {
    seen: Vec<&'a T>,
    messages: MessageCollection,
}

impl<'a, T: WithName> DuplicateChecker<'a, T> {
    fn new() -> DuplicateChecker<'a, T> {
        DuplicateChecker {
            seen: Vec::new(),
            messages: MessageCollection::new(),
        }
    }

    /// checks if an object with the same name was already seen
    /// if an object with the same name already exists the error function is called
    /// the error returned by the function is then stored
    fn check_if_duplicate_exists<F>(&mut self, named: &'a T, error_fn: F)
    where
        F: Fn(&T) -> DatamodelError,
    {
        match self.seen.iter().find(|x| x.name() == named.name()) {
            Some(existing) => self.messages.push_error(error_fn(existing)),
            None => self.seen.push(named),
        }
    }

    fn messages(self) -> MessageCollection {
        self.messages
    }
}

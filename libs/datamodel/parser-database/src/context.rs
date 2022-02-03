mod arguments;
mod attributes;

pub(super) use self::{arguments::Arguments, attributes::Attributes};

use crate::{ast, DatamodelError, Diagnostics, ParserDatabase, ScalarFieldType, ValueValidator};
use std::collections::{HashMap, HashSet};

/// Validation context. This is an implementation detail of ParserDatabase. It
/// contains the database itself, as well as context that is discarded after
/// validation.
pub(crate) struct Context<'a> {
    pub(super) db: &'a mut ParserDatabase,
    pub(super) diagnostics: &'a mut Diagnostics,
    arguments: Arguments,
    attributes: Attributes,

    // @map'ed names indexes. These are not in the db because they are only used for validation.
    pub(super) mapped_model_names: HashMap<&'a str, ast::ModelId>,
    pub(super) mapped_model_scalar_field_names: HashMap<(ast::ModelId, &'a str), ast::FieldId>,
    pub(super) mapped_composite_type_names: HashMap<(ast::CompositeTypeId, &'a str), ast::FieldId>,
    pub(super) mapped_enum_names: HashMap<&'a str, ast::EnumId>,
    pub(super) mapped_enum_value_names: HashMap<(ast::EnumId, &'a str), u32>,
}

impl<'a> Context<'a> {
    pub(super) fn new(db: &'a mut ParserDatabase, diagnostics: &'a mut Diagnostics) -> Self {
        Context {
            db,
            diagnostics,
            arguments: Arguments::default(),
            attributes: Attributes::default(),

            mapped_model_names: Default::default(),
            mapped_model_scalar_field_names: Default::default(),
            mapped_enum_names: Default::default(),
            mapped_enum_value_names: Default::default(),
            mapped_composite_type_names: Default::default(),
        }
    }

    pub(super) fn push_error(&mut self, error: DatamodelError) {
        self.diagnostics.push_error(error)
    }

    pub(super) fn has_errors(&self) -> bool {
        self.diagnostics.has_errors()
    }

    /// We need special code for scalar field attribute validation, because the
    /// attributes on a scalar field are the attributes on the scalar field
    /// itself, plus the attributes on the type alias it may be using. That type
    /// alias may be referencing another type alias that has attributes, and so
    /// on transitively.
    ///
    /// Other than for this peculiarity, this method is identical to
    /// `visit_attributes()`.
    pub(super) fn visit_scalar_field_attributes(
        &mut self,
        model_id: ast::ModelId,
        field_id: ast::FieldId,
        mut scalar_field_type: ScalarFieldType,
        f: impl FnOnce(&mut Attributes<'a>, &mut Context<'a>),
    ) {
        self.attributes.set_attributes(
            self.db.ast[model_id][field_id]
                .attributes
                .iter()
                .enumerate()
                .map(|(attr_idx, attr)| (attr, ast::AttributeId::ModelField(model_id, field_id, attr_idx))),
        );

        while let ScalarFieldType::Alias(alias_id) = scalar_field_type {
            let alias = &self.db.ast[alias_id];
            let attrs = alias
                .attributes
                .iter()
                .enumerate()
                .map(|(attr_idx, attr)| (attr, ast::AttributeId::TypeAlias(alias_id, attr_idx)));
            self.attributes.extend_attributes(attrs);
            scalar_field_type = self.db.types.type_aliases[&alias_id];
        }

        self.visit_attributes_impl(f)
    }

    /// All attribute validation should go through `visit_attributes()`. It lets
    /// us enforce some rules, for example that certain attributes should not be
    /// repeated, and make sure that _all_ attributes are visited during the
    /// validation process, returning unknown attribute errors when it is not
    /// the case.
    ///
    /// This takes a closure so we can better manage ownership of the validation
    /// context and, more importantly, so we can validate at the end of the
    /// closure that all attributes were validated.
    pub(super) fn visit_attributes(
        &mut self,
        ast_attributes: impl ExactSizeIterator<Item = (&'a ast::Attribute, ast::AttributeId)>,
        f: impl FnOnce(&'_ mut Attributes<'a>, &'_ mut Context<'a>),
    ) {
        self.attributes.set_attributes(ast_attributes);
        self.visit_attributes_impl(f)
    }

    fn visit_attributes_impl(&mut self, f: impl FnOnce(&'_ mut Attributes<'a>, &'_ mut Context<'a>)) {
        let mut attributes = std::mem::take(&mut self.attributes);

        f(&mut attributes, self);

        for attribute in attributes.unused_attributes() {
            self.push_error(DatamodelError::new_attribute_not_known_error(
                &attribute.name.name,
                attribute.name.span,
            ))
        }

        self.attributes = attributes; // reuse the allocations.
    }

    /// Implementation detail. Used by `Attributes`.
    fn with_arguments(
        &mut self,
        attribute: &'a ast::Attribute,
        attribute_id: ast::AttributeId,
        f: impl FnOnce(&mut AttributeContext<'_, 'a>),
    ) {
        let mut arguments = match self.arguments.set_attribute(attribute, attribute_id) {
            Ok(()) => std::mem::take(&mut self.arguments), // reuse the allocation for arguments
            Err(mut err) => {
                self.diagnostics.append(&mut err);
                return;
            }
        };

        f(&mut arguments, self);
        arguments.check_for_unused_arguments(&mut self.diagnostics);

        self.arguments = arguments;
    }
}

pub(crate) struct AttributeListContext<'ctx, 'db> {
    attributes: Attributes,
    pub(crate) ctx: &'ctx mut Context<'db>,
}

impl<'ctx, 'db> AttributeListContext<'ctx, 'db> {}

pub(crate) struct AttributeContext<'ctx, 'db> {
    arguments: Arguments,
    pub(crate) ctx: &'ctx mut Context<'db>,
}

impl<'ctx, 'db> AttributeContext<'ctx, 'db> {
    pub(crate) fn optional_arg(&mut self, name: &str) -> Option<ValueValidator<'db>> {
        self.args.remove(&Some(name)).map(|arg| ValueValidator::new(&arg.value))
    }

    /// Gets the arg with the given name, or if it is not found, the first unnamed argument.
    ///
    /// Use this to implement unnamed argument behavior.
    pub(crate) fn default_arg(&mut self, name: &str) -> Result<ValueValidator<'db>, DatamodelError> {
        match (self.args.remove(&Some(name)), self.args.remove(&None)) {
            (Some(arg), None) => Ok(ValueValidator::new(&arg.value)),
            (None, Some(arg)) => Ok(ValueValidator::new(&arg.value)),
            (Some(arg), Some(_)) => Err(DatamodelError::new_duplicate_default_argument_error(name, arg.span)),
            (None, None) => Err(DatamodelError::new_argument_not_found_error(name, self.span())),
        }
    }

    pub(crate) fn new_attribute_validation_error(&self, message: &str) -> DatamodelError {
        DatamodelError::new_attribute_validation_error(message, self.attribute().0.name(), self.span())
    }

    pub(crate) fn optional_default_arg(&mut self, name: &str) -> Option<ValueValidator<'_>> {
        self.default_arg(name).ok()
    }

    pub(crate) fn attribute(&self) -> &'db ast::Attribute {}
}

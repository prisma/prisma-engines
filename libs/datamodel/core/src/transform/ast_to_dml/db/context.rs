pub(super) use attributes::Attributes;

use super::{ParserDatabase, ScalarFieldType};
use crate::{
    ast,
    diagnostics::{DatamodelError, Diagnostics},
    transform::helpers::Arguments,
};
use std::collections::HashSet;

/// Validation context. This is an implementation detail of ParserDatabase. It
/// contains the database itself, as well as context that is discarded after
/// validation.
pub(crate) struct Context<'ast> {
    pub(super) db: ParserDatabase<'ast>,
    pub(super) diagnostics: Diagnostics,
    arguments: Arguments<'ast>,
    attributes: Attributes<'ast>,
}

impl<'ast> Context<'ast> {
    pub(super) fn new(db: ParserDatabase<'ast>, diagnostics: Diagnostics) -> Self {
        Context {
            db,
            diagnostics,
            arguments: Arguments::default(),
            attributes: Attributes::default(),
        }
    }

    pub(super) fn finish(self) -> (ParserDatabase<'ast>, Diagnostics) {
        (self.db, self.diagnostics)
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
        f: impl FnOnce(&'_ mut Attributes<'ast>, &'_ mut Context<'ast>),
    ) {
        self.attributes
            .set_attributes(&self.db.ast[model_id][field_id].attributes);

        while let ScalarFieldType::Alias(alias_id) = scalar_field_type {
            let alias = &self.db.ast[alias_id];
            self.attributes.extend_attributes(&alias.attributes);
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
        ast_attributes: &'ast [ast::Attribute],
        f: impl FnOnce(&'_ mut Attributes<'ast>, &'_ mut Context<'ast>),
    ) {
        self.attributes.set_attributes(ast_attributes);
        self.visit_attributes_impl(f)
    }

    fn visit_attributes_impl(&mut self, f: impl FnOnce(&'_ mut Attributes<'ast>, &'_ mut Context<'ast>)) {
        let mut attributes = std::mem::take(&mut self.attributes);

        f(&mut attributes, self);

        for attribute in attributes.unused_attributes() {
            // Native types...
            if attribute.name.name.contains('.') {
                continue;
            }

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
        attribute: &'ast ast::Attribute,
        f: impl FnOnce(&mut Arguments<'ast>, &mut Context<'ast>),
    ) {
        let mut arguments = match self.arguments.set_attribute(attribute) {
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

mod attributes {
    use super::*;

    #[derive(Default)]
    pub(crate) struct Attributes<'ast> {
        attributes: Vec<&'ast ast::Attribute>,
        unused_attributes: HashSet<usize>,
    }

    impl<'ast> Attributes<'ast> {
        pub(super) fn set_attributes(&mut self, ast_attributes: &'ast [ast::Attribute]) {
            self.attributes.clear();
            self.unused_attributes.clear();
            self.extend_attributes(ast_attributes)
        }

        pub(super) fn extend_attributes(&mut self, ast_attributes: &'ast [ast::Attribute]) {
            self.unused_attributes
                .extend(self.attributes.len()..(self.attributes.len() + ast_attributes.len()));
            self.attributes.extend(ast_attributes.iter());
        }

        pub(super) fn unused_attributes(&self) -> impl Iterator<Item = &'ast ast::Attribute> + '_ {
            self.unused_attributes.iter().map(move |idx| self.attributes[*idx])
        }

        /// Validate an _optional_ attribute that should occur only once.
        pub(crate) fn visit_optional_single<'ctx>(
            &mut self,
            name: &str,
            ctx: &'ctx mut Context<'ast>,
            f: impl FnOnce(&mut Arguments<'ast>, &mut Context<'ast>),
        ) {
            let mut attrs = self.attributes.iter().enumerate().filter(|(_, a)| a.name.name == name);
            let (first_idx, first) = match attrs.next() {
                Some(first) => first,
                None => return, // early return if absent: it's optional
            };

            if attrs.next().is_some() {
                for (idx, attr) in self.attributes.iter().enumerate().filter(|(_, a)| a.name.name == name) {
                    ctx.push_error(DatamodelError::new_duplicate_attribute_error(
                        &attr.name.name,
                        attr.span,
                    ));
                    assert!(self.unused_attributes.remove(&idx));
                }

                return;
            }

            assert!(self.unused_attributes.remove(&first_idx));

            ctx.with_arguments(first, f);
        }

        /// Extract an attribute that can occur zero or more times. Example: @@index on models.
        pub(crate) fn visit_repeated<'ctx>(
            &mut self,
            name: &'static str,
            ctx: &'ctx mut Context<'ast>,
            mut f: impl FnMut(&mut Arguments<'ast>, &mut Context<'ast>),
        ) {
            for (attr_idx, attr) in self
                .attributes
                .iter()
                .enumerate()
                .filter(|(_, attr)| attr.name.name == name)
            {
                ctx.with_arguments(attr, &mut f);
                assert!(self.unused_attributes.remove(&attr_idx));
            }
        }
    }
}

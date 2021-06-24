use super::ParserDatabase;
use crate::{
    ast,
    diagnostics::{DatamodelError, Diagnostics},
    transform::helpers::Arguments,
};
use std::collections::HashSet;

/// Validation context. This is an implementation detail of ParserDatabase. It
/// contains the database itself, as well as context that is discarded after the
/// validation.
pub(super) struct Context<'a, 'ast> {
    pub(super) db: &'a mut ParserDatabase<'ast>,
    diagnostics: &'a mut Diagnostics,
}

impl<'a, 'ast> Context<'a, 'ast> {
    pub(super) fn new(db: &'a mut ParserDatabase<'ast>, diagnostics: &'a mut Diagnostics) -> Self {
        Context { db, diagnostics }
    }

    pub(super) fn push_error(&mut self, error: DatamodelError) {
        self.diagnostics.push_error(error)
    }

    pub(super) fn has_errors(&self) -> bool {
        self.diagnostics.has_errors()
    }

    /// All attribute validation should go through `validate_attributes()`. It
    /// lets us enforce some rules, for example that certain attributes should
    /// not be repeated, and make sure that _all_ attributes are visited during
    /// the validation process, returning unknown attribute errors when it is
    /// not the case.
    pub(super) fn visit_attributes(&mut self, attributes: &'ast [ast::Attribute]) -> Attributes<'ast> {
        Attributes {
            attributes,
            used_attributes: HashSet::with_capacity(attributes.len()),
        }
    }

    /// Implementation detail. Used by `Attributes`.
    fn arguments(&mut self, attribute: &'ast ast::Attribute) -> Option<Arguments<'ast>> {
        match Arguments::new(attribute) {
            Ok(args) => Some(args),
            Err(mut err) => {
                self.diagnostics.append(&mut err);
                None
            }
        }
    }
}

pub(super) struct Attributes<'ast> {
    attributes: &'ast [ast::Attribute],
    used_attributes: HashSet<usize>,
}

impl<'ast> Attributes<'ast> {
    /// Extract an _optional_ attribute that should occur only once.
    pub(super) fn get_optional_single<'ctx>(
        &mut self,
        name: &str,
        ctx: &'ctx mut Context<'_, 'ast>,
    ) -> Option<Arguments<'ast>> {
        let mut attrs = self.attributes.iter().enumerate().filter(|(_, a)| a.name.name == name);
        let (first_idx, first) = attrs.next()?; // early return if absent: it's optional

        if let Some((_, next)) = attrs.next() {
            ctx.push_error(DatamodelError::new_duplicate_attribute_error(
                &next.name.name,
                next.name.span,
            ));
            return None;
        }

        self.used_attributes.insert(first_idx);
        ctx.arguments(first)
    }
}

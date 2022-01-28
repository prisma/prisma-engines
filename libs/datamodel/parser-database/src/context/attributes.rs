use super::*;

#[derive(Default)]
pub(crate) struct Attributes<'ast> {
    attributes: Vec<(&'ast ast::Attribute, ast::AttributeId)>,
    unused_attributes: HashSet<usize>,
}

impl<'ast> Attributes<'ast> {
    pub(super) fn set_attributes(
        &mut self,
        ast_attributes: impl ExactSizeIterator<Item = (&'ast ast::Attribute, ast::AttributeId)>,
    ) {
        self.attributes.clear();
        self.unused_attributes.clear();
        self.extend_attributes(ast_attributes)
    }

    pub(super) fn extend_attributes(
        &mut self,
        ast_attributes: impl ExactSizeIterator<Item = (&'ast ast::Attribute, ast::AttributeId)>,
    ) {
        self.unused_attributes
            .extend(self.attributes.len()..(self.attributes.len() + ast_attributes.len()));
        for attr in ast_attributes {
            self.attributes.push(attr);
        }
    }

    pub(super) fn unused_attributes(&self) -> impl Iterator<Item = &'ast ast::Attribute> + '_ {
        self.unused_attributes.iter().map(move |idx| self.attributes[*idx].0)
    }

    /// Validate an _optional_ attribute that should occur only once.
    pub(crate) fn visit_optional_single<'ctx>(
        &mut self,
        name: &str,
        ctx: &'ctx mut Context<'ast>,
        f: impl FnOnce(&mut Arguments<'ast>, &mut Context<'ast>),
    ) {
        let mut attrs = self
            .attributes
            .iter()
            .enumerate()
            .filter(|(_, (a, _))| a.name.name == name);
        let (first_idx, first) = match attrs.next() {
            Some(first) => first,
            None => return, // early return if absent: it's optional
        };

        if attrs.next().is_some() {
            for (idx, (attr, _attr_id)) in self
                .attributes
                .iter()
                .enumerate()
                .filter(|(_, (a, _))| a.name.name == name)
            {
                ctx.push_error(DatamodelError::new_duplicate_attribute_error(
                    &attr.name.name,
                    attr.span,
                ));
                assert!(self.unused_attributes.remove(&idx));
            }

            return;
        }

        assert!(self.unused_attributes.remove(&first_idx));

        ctx.with_arguments(first.0, first.1, f);
    }

    /// Extract an attribute that can occur zero or more times. Example: @@index on models.
    pub(crate) fn visit_repeated<'ctx>(
        &mut self,
        name: &'static str,
        ctx: &'ctx mut Context<'ast>,
        mut f: impl FnMut(&mut Arguments<'ast>, &mut Context<'ast>),
    ) {
        for (attr_idx, (attr, attr_id)) in self
            .attributes
            .iter()
            .enumerate()
            .filter(|(_, (attr, _))| attr.name.name == name)
        {
            ctx.with_arguments(attr, *attr_id, &mut f);
            assert!(self.unused_attributes.remove(&attr_idx));
        }
    }

    /// Look for an optional attribute with a name of the form
    /// "<datasource_name>.<attribute_name>", and call the passed-in function
    /// with the scope name, attribute name and the arguments.
    ///
    /// Also note that native type arguments are treated differently from
    /// arguments to other attributes: everywhere else, attributes are named,
    /// with a default that can be first, but with native types, arguments are
    /// purely positional.
    pub(crate) fn visit_datasource_scoped<'ctx>(
        &mut self,
        ctx: &'ctx mut Context<'ast>,
        f: impl FnOnce(&'ast str, &'ast str, &'ast ast::Attribute, &mut Context<'ast>),
    ) {
        let attrs = self
            .attributes
            .iter()
            .enumerate()
            .filter(|(_, (attr, _))| attr.name.name.contains('.'));
        let mut native_type_attr = None;

        // Extract the attribute, validating that there are no duplicates.
        for (attr_idx, (attr, _)) in attrs {
            assert!(self.unused_attributes.remove(&attr_idx));

            match attr.name.name.split_once('.') {
                None => unreachable!(),
                Some((ds, attr_name)) => {
                    if native_type_attr.replace((ds, attr, attr_name)).is_some() {
                        ctx.push_error(DatamodelError::new_duplicate_attribute_error(ds, attr.span));
                    }
                }
            }
        }

        let (ds, attr, attr_name) = match native_type_attr {
            Some(attr) => attr,
            None => return, // early return if absent: it's optional
        };

        f(ds, attr_name, attr, ctx);
    }
}

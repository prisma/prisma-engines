use super::*;
use datamodel_connector::connector_error::{ConnectorError, ErrorKind};

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

    /// Look for an optional attribute with a name of the form
    /// "<datasource_name>.<attribute_name>", and call the passed-in function
    /// with the attribute name and the arguments.
    ///
    /// Also note that native type arguments are treated differently from
    /// arguments to other attributes: everywhere else, attributes are named,
    /// with a default that can be first, but with native types, arguments are
    /// purely positional.
    pub(crate) fn visit_datasource_scoped<'ctx>(
        &mut self,
        ctx: &'ctx mut Context<'ast>,
        f: impl FnOnce(&'ast str, &'ast ast::Attribute, &mut Context<'ast>),
    ) {
        let datasource = if let Some(ds) = ctx.db.datasource() { ds } else { return };

        let attrs = self
            .attributes
            .iter()
            .enumerate()
            .filter(|(_, attr)| attr.name.name.contains('.'));
        let mut native_type_attr = None;

        // Extract the attribute, validating that:
        //
        // 1. All scoped attributes are scoped with the right datasource name
        // 2. There are no duplicates
        for (attr_idx, attr) in attrs {
            assert!(self.unused_attributes.remove(&attr_idx));

            match attr.name.name.split_once('.') {
                None => unreachable!(),
                Some((ds, attr_name)) if ds == datasource.name => {
                    if native_type_attr.replace((attr, attr_name)).is_some() {
                        ctx.push_error(DatamodelError::new_duplicate_attribute_error(ds, attr.span));
                    }
                }
                Some((bad_datasource, attr_name)) => {
                    ctx.push_error(DatamodelError::new_connector_error(
                        &ConnectorError::from_kind(ErrorKind::InvalidPrefixForNativeTypes {
                            given_prefix: bad_datasource.to_owned(),
                            expected_prefix: datasource.name.clone(),
                            suggestion: [datasource.name.as_str(), attr_name].join("."),
                        })
                        .to_string(),
                        attr.span,
                    ));
                }
            }
        }

        let (attr, attr_name) = match native_type_attr {
            Some(attr) => attr,
            None => return, // early return if absent: it's optional
        };

        f(attr_name, attr, ctx);
    }
}

use super::{Args, DirectiveValidator, Error};
use crate::ast;
use crate::dml;
use crate::errors::ValidationError;

/// Moves an directive into a namespace scope.
///
/// This is mainly used with custom source blocks. It wraps a directive and
/// preprends the source name in front of the directive name.
pub struct DirectiveScope<T> {
    inner: Box<dyn DirectiveValidator<T>>,
    #[allow(dead_code)]
    scope: String,
    name: String,
}

impl<T> DirectiveScope<T> {
    /// Creates a new instance, using the given directive and
    /// a namespae name.
    pub fn new(inner: Box<dyn DirectiveValidator<T>>, scope: &str) -> DirectiveScope<T> {
        DirectiveScope {
            name: format!("{}.{}", scope, inner.directive_name()),
            inner,
            scope: String::from(scope),
        }
    }
}

impl<T> DirectiveValidator<T> for DirectiveScope<T> {
    fn directive_name(&self) -> &str {
        &self.name
    }
    fn validate_and_apply(&self, args: &mut Args, obj: &mut T) -> Result<(), Error> {
        self.inner.validate_and_apply(args, obj)
    }
    fn serialize(&self, obj: &T, datamodel: &dml::Datamodel) -> Result<Vec<ast::Directive>, Error> {
        self.inner.serialize(obj, datamodel)
    }
}

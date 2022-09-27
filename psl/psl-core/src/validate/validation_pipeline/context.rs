use crate::{
    datamodel_connector::{Connector, ReferentialIntegrity},
    Datasource, PreviewFeature,
};
use diagnostics::{DatamodelError, Diagnostics};
use enumflags2::BitFlags;

/// The validation context. The lifetime parameter is _not_ the AST lifetime, but the subtype of
/// all relevant lifetimes. No data escapes for validations, so the context only need to be valid
/// for the duration of validations.
pub(crate) struct Context<'a> {
    pub(super) db: &'a parser_database::ParserDatabase,
    pub(super) datasource: Option<&'a Datasource>,
    pub(super) preview_features: BitFlags<PreviewFeature>,
    pub(super) connector: &'static dyn Connector,
    /// Referential integrity is a pure function of the datasource, but since there are defaults,
    /// it's more consistent to resolve it once, here.
    pub(super) referential_integrity: ReferentialIntegrity,
    pub(super) diagnostics: &'a mut Diagnostics,
}

impl Context<'_> {
    /// Pure convenience method. Forwards to Diagnostics::push_error().
    pub(super) fn push_error(&mut self, error: DatamodelError) {
        self.diagnostics.push_error(error);
    }
}

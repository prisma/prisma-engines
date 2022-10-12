mod context;
mod validations;

use crate::{
    configuration,
    datamodel_connector::{Connector, EmptyDatamodelConnector, RelationMode},
    diagnostics::Diagnostics,
    PreviewFeature,
};
use enumflags2::BitFlags;
use parser_database::ParserDatabase;

pub struct ValidateOutput {
    pub(crate) db: ParserDatabase,
    pub(crate) diagnostics: Diagnostics,
    pub(crate) relation_mode: RelationMode,
    pub(crate) connector: &'static dyn Connector,
}

/// Validate a Prisma schema.
pub(crate) fn validate(
    db: ParserDatabase,
    sources: &[configuration::Datasource],
    preview_features: BitFlags<PreviewFeature>,
    diagnostics: Diagnostics,
) -> ValidateOutput {
    let source = sources.first();
    let connector = source.map(|s| s.active_connector).unwrap_or(&EmptyDatamodelConnector);
    let relation_mode = source.map(|s| s.relation_mode()).unwrap_or_default();

    let mut output = ValidateOutput {
        db,
        diagnostics,
        relation_mode,
        connector,
    };

    // Early return so that the validator does not have to deal with invalid schemas
    if !output.diagnostics.errors().is_empty() {
        return output;
    }

    let mut context = context::Context {
        db: &output.db,
        datasource: source,
        preview_features,
        connector,
        relation_mode,
        diagnostics: &mut output.diagnostics,
    };

    validations::validate(&mut context);

    output
}

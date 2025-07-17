mod context;
mod validations;

use crate::{
    PreviewFeature, configuration,
    datamodel_connector::{Connector, EmptyDatamodelConnector, RelationMode},
    diagnostics::Diagnostics,
};
use enumflags2::BitFlags;
use parser_database::ParserDatabase;

pub struct ParseOutput {
    pub(crate) db: ParserDatabase,
    pub(crate) relation_mode: RelationMode,
    pub(crate) connector: &'static dyn Connector,
}

/// Parse a Prisma schema, but skip validations.
pub(crate) fn parse_without_validation(db: ParserDatabase, sources: &[configuration::Datasource]) -> ParseOutput {
    let source = sources.first();
    let connector = source.map(|s| s.active_connector).unwrap_or(&EmptyDatamodelConnector);
    let relation_mode = source.map(|s| s.relation_mode()).unwrap_or_default();

    ParseOutput {
        db,
        relation_mode,
        connector,
    }
}

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
    let ParseOutput {
        connector,
        relation_mode,
        db,
    } = parse_without_validation(db, sources);

    let mut output = ValidateOutput {
        connector,
        relation_mode,
        db,
        diagnostics,
    };

    // Early return so that the validator does not have to deal with invalid schemas
    if !output.diagnostics.errors().is_empty() {
        return output;
    }

    let source = sources.first();

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

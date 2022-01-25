mod context;
mod validations;

use crate::{ast, common::preview_features::PreviewFeature, configuration, diagnostics::Diagnostics};
use datamodel_connector::{Connector, EmptyDatamodelConnector, ReferentialIntegrity};
use enumflags2::BitFlags;
use parser_database::ParserDatabase;

pub struct ValidateOutput<'ast> {
    pub(crate) db: ParserDatabase<'ast>,
    pub(crate) diagnostics: Diagnostics,
    pub(crate) referential_integrity: ReferentialIntegrity,
    pub(crate) connector: &'static dyn Connector,
}

/// Analyze and validate a schema AST.
///
/// This will attempt to
///
/// * Resolve all attributes
/// * Resolve and check default values
/// * Resolve and check all field types
/// * ...
/// * Validate the schema
pub(crate) fn validate<'ast>(
    src: &'ast str,
    ast_schema: &'ast ast::SchemaAst,
    sources: &[configuration::Datasource],
    preview_features: BitFlags<PreviewFeature>,
    diagnostics: Diagnostics,
) -> ValidateOutput<'ast> {
    let source = sources.first();
    let connector = source.map(|s| s.active_connector).unwrap_or(&EmptyDatamodelConnector);
    let referential_integrity = source.map(|s| s.referential_integrity()).unwrap_or_default();

    // Make sense of the AST.
    let (db, diagnostics) = ParserDatabase::new(src, ast_schema, diagnostics);

    let mut output = ValidateOutput {
        db,
        diagnostics,
        referential_integrity,
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
        referential_integrity,
        diagnostics: &mut output.diagnostics,
    };

    validations::validate(&mut context);

    output
}

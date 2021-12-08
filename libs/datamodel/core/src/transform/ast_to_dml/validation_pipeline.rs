mod context;
mod validations;

use super::{db::ParserDatabase, lift::LiftAstToDml};
use crate::{
    ast, common::preview_features::PreviewFeature, configuration, diagnostics::Diagnostics, ValidatedDatamodel,
};
use datamodel_connector::EmptyDatamodelConnector;
use enumflags2::BitFlags;

/// Validates an AST semantically and promotes it to a datamodel/schema.
///
/// This will attempt to
///
/// * Resolve all attributes
/// * Resolve and check default values
/// * Resolve and check all field types
/// * Validate the schema
pub(crate) fn validate(
    ast_schema: &ast::SchemaAst,
    sources: &[configuration::Datasource],
    preview_features: BitFlags<PreviewFeature>,
    relation_transformation_enabled: bool,
) -> Result<ValidatedDatamodel, Diagnostics> {
    let source = sources.first();
    let diagnostics = Diagnostics::new();
    let connector = source.map(|s| s.active_connector).unwrap_or(&EmptyDatamodelConnector);
    let referential_integrity = source.map(|s| s.referential_integrity()).unwrap_or_default();

    // Make sense of the AST.
    let (db, mut diagnostics) = ParserDatabase::new(ast_schema, diagnostics);

    // Early return so that the validator does not have to deal with invalid schemas
    diagnostics.to_result()?;

    let mut context = context::Context {
        db: &db,
        datasource: source,
        preview_features,
        connector,
        referential_integrity,
        diagnostics: &mut diagnostics,
    };

    validations::validate(&mut context, relation_transformation_enabled);
    diagnostics.to_result()?;

    let schema = LiftAstToDml::new(&db, connector, referential_integrity).lift();

    Ok(ValidatedDatamodel {
        subject: schema,
        warnings: diagnostics.warnings().to_owned(),
    })
}

mod validations;

use super::{db::ParserDatabase, lift::LiftAstToDml, validate::Validator};
use crate::{
    ast, common::preview_features::PreviewFeature, configuration, diagnostics::Diagnostics, ValidatedDatamodel,
};
use enumflags2::BitFlags;

/// Is responsible for loading and validating the Datamodel defined in an AST.
/// Wrapper for all lift and validation steps
pub(crate) struct ValidationPipeline<'a> {
    source: Option<&'a configuration::Datasource>,
    validator: Validator<'a>,
    preview_features: BitFlags<PreviewFeature>,
}

impl<'a, 'b> ValidationPipeline<'a> {
    pub(crate) fn new(
        sources: &'a [configuration::Datasource],
        preview_features: BitFlags<PreviewFeature>,
    ) -> ValidationPipeline<'a> {
        let source = sources.first();

        ValidationPipeline {
            source,
            validator: Validator::new(source),
            preview_features,
        }
    }

    /// Validates an AST semantically and promotes it to a datamodel/schema.
    ///
    /// This method will attempt to
    /// * Resolve all attributes
    /// * Recursively evaluate all functions
    /// * Perform string interpolation
    /// * Resolve and check default values
    /// * Resolve and check all field types
    pub(crate) fn validate(
        &self,
        ast_schema: &ast::SchemaAst,
        relation_transformation_enabled: bool,
    ) -> Result<ValidatedDatamodel, Diagnostics> {
        let diagnostics = Diagnostics::new();

        // Phase 0 is parsing.
        // Phase 1 is source block loading.

        // Phase 2: Make sense of the AST.
        let (db, mut diagnostics) = ParserDatabase::new(ast_schema, self.source, diagnostics, self.preview_features);

        // Early return so that the validator does not have to deal with invalid schemas
        diagnostics.to_result()?;

        // Phase 3: Global validations after we have consistent data model.
        validations::validate(&db, &mut diagnostics, relation_transformation_enabled);
        diagnostics.to_result()?;

        // Phase 4: Lift AST to DML. This can't fail.
        let schema = LiftAstToDml::new(&db).lift();

        // From now on we do not operate on the internal ast anymore, but DML.
        // Please try to avoid all new validations after this, if you can.

        // Phase 5: Validation (deprecated, move stuff out from here if you can)
        self.validator.validate(db.ast(), &schema, &mut diagnostics);

        // Phase 6: Post Standardisation Validation (deprecated, move stuff out from here if you can)
        self.validator
            .post_standardisation_validate(ast_schema, &schema, &mut diagnostics);

        diagnostics.to_result()?;

        Ok(ValidatedDatamodel {
            subject: schema,
            warnings: diagnostics.warnings().to_owned(),
        })
    }
}

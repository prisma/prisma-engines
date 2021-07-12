use super::db::ParserDatabase;
use super::*;
use crate::{
    ast, common::preview_features::PreviewFeature, configuration, diagnostics::Diagnostics,
    transform::ast_to_dml::standardise_parsing::StandardiserForParsing, ValidatedDatamodel,
};
use enumflags2::BitFlags;

/// Is responsible for loading and validating the Datamodel defined in an AST.
/// Wrapper for all lift and validation steps
pub struct ValidationPipeline<'a> {
    source: Option<&'a configuration::Datasource>,
    validator: Validator<'a>,
    standardiser_for_formatting: StandardiserForFormatting,
    standardiser_for_parsing: StandardiserForParsing,
}

impl<'a, 'b> ValidationPipeline<'a> {
    pub fn new(
        sources: &'a [configuration::Datasource],
        preview_features: BitFlags<PreviewFeature>,
    ) -> ValidationPipeline<'a> {
        let source = sources.first();

        ValidationPipeline {
            source,
            validator: Validator::new(source, preview_features),
            standardiser_for_formatting: StandardiserForFormatting::new(),
            standardiser_for_parsing: StandardiserForParsing::new(preview_features),
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
    pub fn validate(
        &self,
        ast_schema: &ast::SchemaAst,
        relation_transformation_enabled: bool,
    ) -> Result<ValidatedDatamodel, Diagnostics> {
        let diagnostics = Diagnostics::new();

        // Phase 0 is parsing.
        // Phase 1 is source block loading.

        // Phase 2: Name resolution.
        let (db, mut diagnostics) = ParserDatabase::new(ast_schema, self.source, diagnostics);

        // Early return so that the validator does not have to deal with invalid schemas
        diagnostics.to_result()?;

        // Phase 3: Lift AST to DML.
        let mut schema = LiftAstToDml::new(&db, &mut diagnostics).lift();

        // Cannot continue on lifter error.
        diagnostics.to_result()?;

        // Phase 4: Validation
        self.validator.validate(&db, &mut schema, &mut diagnostics);

        // Early return so that the standardiser does not have to deal with invalid schemas
        diagnostics.to_result()?;

        // TODO: Move consistency stuff into different module.
        // Phase 5: Consistency fixes. These don't fail and always run, during parsing AND formatting
        if let Err(mut err) = self.standardiser_for_parsing.standardise(&mut schema) {
            diagnostics.append(&mut err);
        }

        // Transform phase: These only run during formatting.
        if relation_transformation_enabled {
            if let Err(mut err) = self.standardiser_for_formatting.standardise(ast_schema, &mut schema) {
                diagnostics.append(&mut err);
            }
        }

        // Early return so that the post validation does not have to deal with invalid schemas
        diagnostics.to_result()?;

        // Phase 6: Post Standardisation Validation
        if let Err(mut err) = self.validator.post_standardisation_validate(ast_schema, &mut schema) {
            diagnostics.append(&mut err);
        }

        diagnostics.to_result()?;

        Ok(ValidatedDatamodel {
            subject: schema,
            warnings: diagnostics.warnings().to_owned(),
        })
    }
}

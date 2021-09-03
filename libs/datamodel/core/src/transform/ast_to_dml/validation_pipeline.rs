use super::{
    db::ParserDatabase, lift::LiftAstToDml, standardise_formatting::StandardiserForFormatting, standardise_parsing,
    validate::Validator,
};
use crate::{
    ast, common::preview_features::PreviewFeature, configuration, diagnostics::Diagnostics, ValidatedDatamodel,
};
use enumflags2::BitFlags;

/// Is responsible for loading and validating the Datamodel defined in an AST.
/// Wrapper for all lift and validation steps
pub(crate) struct ValidationPipeline<'a> {
    source: Option<&'a configuration::Datasource>,
    validator: Validator<'a>,
    standardiser_for_formatting: StandardiserForFormatting,
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
            standardiser_for_formatting: StandardiserForFormatting::new(),
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

        // Phase 3: Lift AST to DML. This can't fail.
        let mut schema = LiftAstToDml::new(&db).lift();

        // Phase 4: Validation
        self.validator.validate(db.ast(), &schema, &mut diagnostics);

        // Early return so that the standardiser does not have to deal with invalid schemas
        diagnostics.to_result()?;

        // Phase 5: Consistency fixes. These don't fail and always run, during parsing AND formatting
        standardise_parsing::standardise(&mut schema);

        // Transform phase: These only run during formatting.
        if relation_transformation_enabled {
            if let Err(mut err) = self.standardiser_for_formatting.standardise(ast_schema, &mut schema) {
                diagnostics.append(&mut err);
                // Early return so that the post validation does not have to deal with invalid schemas
                return Err(diagnostics);
            }
        }

        // Phase 6: Post Standardisation Validation
        self.validator
            .post_standardisation_validate(ast_schema, &schema, &mut diagnostics);

        diagnostics.to_result()?;

        Ok(ValidatedDatamodel {
            subject: schema,
            warnings: diagnostics.warnings().to_owned(),
        })
    }
}

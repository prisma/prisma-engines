use super::db::ParserDatabase;
use super::*;
use crate::common::datamodel_context::DatamodelContext;
use crate::transform::ast_to_dml::standardise_parsing::StandardiserForParsing;
use crate::{ast, diagnostics::Diagnostics, ValidatedDatamodel};

/// Is responsible for loading and validating the Datamodel defined in an AST.
/// Wrapper for all lift and validation steps
pub struct ValidationPipeline<'a> {
    context: &'a DatamodelContext,
    validator: Validator<'a>,
    standardiser_for_parsing: StandardiserForParsing,
    standardiser_for_formatting: StandardiserForFormatting,
}

impl<'a, 'b> ValidationPipeline<'a> {
    pub fn new(context: &'a DatamodelContext) -> ValidationPipeline<'a> {
        ValidationPipeline {
            context,
            validator: Validator::new(&context),
            standardiser_for_formatting: StandardiserForFormatting::new(),
            standardiser_for_parsing: StandardiserForParsing::new(),
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
        let mut diagnostics = Diagnostics::new();

        // Phase 0 is parsing.
        // Phase 1 is source block loading.

        // Phase 2: Name resolution.
        let db = ParserDatabase::new(ast_schema, &mut diagnostics);

        // Early return so that the validator does not have to deal with invalid schemas
        diagnostics.make_result()?;

        // Phase 3: Lift AST to DML.
        let lifter = LiftAstToDml::new(&self.context, &db);

        let mut schema = match lifter.lift() {
            Err(err) => {
                // Cannot continue on lifter error.
                diagnostics.extend(err);
                return Err(diagnostics);
            }
            Ok(schema) => schema,
        };

        // Phase 4: Validation
        if let Err(err) = self.validator.validate(&db, &mut schema) {
            diagnostics.extend(err);
        }

        // Early return so that the standardiser does not have to deal with invalid schemas
        diagnostics.make_result()?;

        // TODO: Move consistency stuff into different module.
        // Phase 5: Consistency fixes. These don't fail and always run, during parsing AND formatting
        if let Err(err) = self.standardiser_for_parsing.standardise(&mut schema) {
            diagnostics.extend(err);
        }

        // Transform phase: These only run during formatting.
        if relation_transformation_enabled {
            if let Err(err) = self.standardiser_for_formatting.standardise(ast_schema, &mut schema) {
                diagnostics.extend(err);
            }
        }
        // Early return so that the post validation does not have to deal with invalid schemas
        diagnostics.make_result()?;

        // Phase 6: Post Standardisation Validation
        if let Err(err) = self.validator.post_standardisation_validate(ast_schema, &mut schema) {
            diagnostics.extend(err);
        }

        if diagnostics.has_errors() {
            Err(diagnostics)
        } else {
            Ok(ValidatedDatamodel {
                subject: schema,
                warnings: diagnostics.warnings,
            })
        }
    }
}

use std::marker::PhantomData;

use dml::datamodel::Datamodel;
use enumflags2::BitFlags;

use crate::{
    ast,
    transform::ast_to_dml::{DatasourceLoader, GeneratorLoader, ValidationPipeline},
    Configuration, Diagnostics, Validated, ValidatedConfiguration, ValidationFeature,
};

#[derive(Debug, Clone)]
pub struct Validator<T>
where
    T: Sized,
{
    flags: BitFlags<ValidationFeature>,
    datasource_url_overrides: Vec<(String, String)>,
    _phantom: PhantomData<T>,
}

impl<T> Validator<T> {
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
            flags: BitFlags::empty(),
            datasource_url_overrides: Vec::new(),
        }
    }

    /// Override the datasource URL in the given datasource.
    pub fn datasource_url_override(&mut self, datasource_name: impl ToString, datasource_url: impl ToString) {
        self.datasource_url_overrides
            .push((datasource_name.to_string(), datasource_url.to_string()));
    }

    /// Do not validate uris in the datasource part.
    pub fn ignore_datasource_urls(&mut self) {
        self.flags.insert(ValidationFeature::IgnoreDatasourceUrls)
    }
}

impl Validator<Datamodel> {
    /// Run the standardizer to modify the data model.
    pub fn standardize_models(&mut self) {
        self.flags.insert(ValidationFeature::StandardizeModels)
    }

    /// Parse a data model string into a validated data model.
    pub fn parse_str(&self, schema: &str) -> Result<Validated<Datamodel>, Diagnostics> {
        let mut diagnostics = Diagnostics::new();
        let ast = ast::parser::parse_schema(schema)?;

        let source_loader = DatasourceLoader::new(self.flags);
        let sources = source_loader.load_datasources_from_ast(&ast, self.datasource_url_overrides.clone())?;

        let generators = GeneratorLoader::load_generators_from_ast(&ast)?;
        let validator = ValidationPipeline::new(&sources.subject, self.flags);

        diagnostics.append_warning_vec(sources.warnings);
        diagnostics.append_warning_vec(generators.warnings);

        match validator.validate(&ast) {
            Ok(mut src) => {
                src.warnings.append(&mut diagnostics.warnings);
                Ok(src)
            }
            Err(mut err) => {
                diagnostics.append(&mut err);
                Err(diagnostics)
            }
        }
    }

    /// Validates a [Schema AST](/ast/struct.SchemaAst.html) and returns its
    /// [Datamodel](/struct.Datamodel.html).
    pub fn lift_ast(&self, ast: &ast::SchemaAst) -> Result<Validated<Datamodel>, Diagnostics> {
        // we are not interested in the sources in this case. Hence we can ignore the datasource urls.
        let flags = self.flags & ValidationFeature::IgnoreDatasourceUrls;

        let mut diagnostics = Diagnostics::new();
        let source_loader = DatasourceLoader::new(flags);
        let sources = source_loader.load_datasources_from_ast(ast, self.datasource_url_overrides.clone())?;

        let generators = GeneratorLoader::load_generators_from_ast(&ast)?;
        let validator = ValidationPipeline::new(&sources.subject, self.flags);

        diagnostics.append_warning_vec(sources.warnings);
        diagnostics.append_warning_vec(generators.warnings);

        match validator.validate(&ast) {
            Ok(mut src) => {
                src.warnings.append(&mut diagnostics.warnings);
                Ok(src)
            }
            Err(mut err) => {
                diagnostics.append(&mut err);
                Err(diagnostics)
            }
        }
    }
}

impl Validator<Configuration> {
    /// Parse a data model string into a validated configuration.
    pub fn parse_str(&self, schema: &str) -> Result<Validated<Configuration>, Diagnostics> {
        let mut warnings = Vec::new();
        let ast = ast::parser::parse_schema(schema)?;
        let source_loader = DatasourceLoader::new(self.flags);

        let mut validated_sources =
            source_loader.load_datasources_from_ast(&ast, self.datasource_url_overrides.clone())?;
        let mut validated_generators = GeneratorLoader::load_generators_from_ast(&ast)?;

        warnings.append(&mut validated_generators.warnings);
        warnings.append(&mut validated_sources.warnings);

        Ok(ValidatedConfiguration {
            subject: Configuration {
                datasources: validated_sources.subject,
                generators: validated_generators.subject,
            },
            warnings,
        })
    }
}

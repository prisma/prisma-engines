use super::super::helpers::*;
use crate::{ast, configuration::Encryptor, diagnostics::*};
use std::collections::HashMap;

const PROVIDER_KEY: &str = "provider";
const TOKEN_KEY: &str = "token";
const FIRST_CLASS_PROPERTIES: &[&str] = &[PROVIDER_KEY, TOKEN_KEY];

/// Is responsible for loading and validating Generators defined in an AST.
pub struct EncryptorLoader {}

impl EncryptorLoader {
    pub fn load_encryptors_from_ast(ast_schema: &ast::SchemaAst) -> Result<ValidatedEncryptors, Diagnostics> {
        let mut encryptors: Vec<Encryptor> = vec![];
        let mut diagnostics = Diagnostics::new();

        for encryptor in &ast_schema.encryptors() {
            match Self::lift_encryptor(&encryptor) {
                Ok(loaded_gen) => {
                    diagnostics.append_warning_vec(loaded_gen.warnings);
                    encryptors.push(loaded_gen.subject)
                }
                // Lift error.
                Err(err) => {
                    for e in err.errors {
                        match e {
                            DatamodelError::ArgumentNotFound { argument_name, span } => {
                                diagnostics.push_error(DatamodelError::new_generator_argument_not_found_error(
                                    argument_name.as_str(),
                                    encryptor.name.name.as_str(),
                                    span,
                                ));
                            }
                            _ => {
                                diagnostics.push_error(e);
                            }
                        }
                    }
                    diagnostics.append_warning_vec(err.warnings)
                }
            }
        }

        if diagnostics.has_errors() {
            Err(diagnostics)
        } else {
            Ok(ValidatedEncryptors {
                subject: encryptors,
                warnings: diagnostics.warnings,
            })
        }
    }

    fn lift_encryptor(ast_encryptor: &ast::EncryptorConfig) -> Result<ValidatedEncryptor, Diagnostics> {
        let mut args = Arguments::new(&ast_encryptor.properties, ast_encryptor.span);
        let mut diagnostics = Diagnostics::new();

        let provider = args.arg(PROVIDER_KEY)?.as_str()?;
        let token = if let Ok(arg) = args.arg(TOKEN_KEY) {
            Some(arg.as_str()?)
        } else {
            None
        };

        let mut properties: HashMap<String, String> = HashMap::new();

        for prop in &ast_encryptor.properties {
            let is_first_class_prop = FIRST_CLASS_PROPERTIES.iter().any(|k| *k == prop.name.name);
            if is_first_class_prop {
                continue;
            }

            properties.insert(prop.name.name.clone(), prop.value.to_string());
        }

        Ok(ValidatedEncryptor {
            subject: Encryptor {
                name: ast_encryptor.name.name.clone(),
                provider,
                token,
                config: properties,
                documentation: ast_encryptor.documentation.clone().map(|comment| comment.text),
            },
            warnings: diagnostics.warnings,
        })
    }
}

use crate::{PrismaError, PrismaResult};
use serde::Deserialize;
use serde_json;

/// Loads data model components for the v2 data model.
/// The v2 data model is provided either as file (PRISMA_DML_PATH) or as string in the env (PRISMA_DML).
/// Attempts to construct a Prisma v2 datamodel.
/// Returns: DatamodelV2Components
///     Err      If a source for v2 was found, but conversion failed.
///     Ok(Some) If a source for v2 was found, and the conversion suceeded.
///     Ok(None) If no source for a v2 data model was found.
pub fn load(
    dml_string: &str,
    datasource_overwrites: Option<String>,
    ignore_env_var_errors: bool,
) -> PrismaResult<datamodel::Configuration> {
    let config_result = if ignore_env_var_errors {
        datamodel::parse_configuration_and_ignore_env_errors(&dml_string)
    } else {
        datamodel::parse_configuration(&dml_string)
    };

    match config_result {
        Err(errors) => Err(PrismaError::ConversionError(errors, dml_string.to_string())),
        Ok(mut configuration) => {
            if let Some(overwrites) = datasource_overwrites {
                let datasource_overwrites: Vec<SourceOverride> = serde_json::from_str(&overwrites)?;

                for datasource_override in datasource_overwrites {
                    for datasource in &mut configuration.datasources {
                        if &datasource_override.name == datasource.name() {
                            debug!(
                                "overwriting datasource {} with url {}",
                                &datasource_override.name, &datasource_override.url
                            );
                            datasource.set_url(&datasource_override.url);
                        }
                    }
                }
            }
            Ok(configuration)
        }
    }
}

#[derive(Deserialize)]
struct SourceOverride {
    name: String,
    url: String,
}

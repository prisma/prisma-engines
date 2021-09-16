use datamodel::diagnostics::ValidatedConfiguration;

mod datasources;
mod generators;
mod nice_warnings;
mod sources;

pub fn parse_config(schema: &str) -> Result<ValidatedConfiguration, String> {
    datamodel::parse_configuration(schema).map_err(|err| err.to_pretty_string("schema.prisma", schema))
}

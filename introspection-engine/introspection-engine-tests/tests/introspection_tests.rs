mod add_prisma1_defaults;
mod commenting_out;
mod enums;
mod identify_version;
mod lists;
mod model_renames;
mod native_types;
mod re_introspection;
mod relations;
mod relations_with_compound_fk;
mod remapping_database_names;
mod rpc_calls;
mod tables;

pub type TestResult = eyre::Result<()>;

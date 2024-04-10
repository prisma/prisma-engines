use psl::SourceFile;
use serde::Deserialize;

/// Struct for supporting multiple files
/// in a backward-compatible way: can either accept
/// a single file contents or vector of (filePath, content) tuples.
/// Can be converted to the input for `psl::validate_multi_file` from
/// any of the variants.
#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub(crate) enum SchemaFileInput {
    Single(String),
    Multiple(Vec<(String, SourceFile)>),
}

impl From<SchemaFileInput> for Vec<(String, SourceFile)> {
    fn from(value: SchemaFileInput) -> Self {
        match value {
            SchemaFileInput::Single(content) => vec![("schema.prisma".to_owned(), content.into())],
            SchemaFileInput::Multiple(file_list) => file_list,
        }
    }
}

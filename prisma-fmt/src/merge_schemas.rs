use psl::schema_ast::ast::SchemaAst;
use serde::Deserialize;

use crate::schema_file_input::SchemaFileInput;

#[derive(Debug, Deserialize)]
pub struct MergeSchemasParams {
    schema: SchemaFileInput,
}

pub(crate) fn merge_schemas(params: &str) -> String {
    let params: MergeSchemasParams = match serde_json::from_str(params) {
        Ok(params) => params,
        Err(serde_err) => {
            panic!("Failed to deserialize GetDmmfParams: {serde_err}");
        }
    };

    let schema = psl::validate_multi_file(params.schema.into());
    let merged_ast = SchemaAst::merge(schema.db.into_iter_asts());

    unimplemented!()
}

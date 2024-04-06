mod actions;
mod code_actions;
mod get_config;
mod get_dmmf;
mod lint;
mod native;
mod preview;
mod text_document_completion;
mod validate;
mod offsets;

use log::*;
pub use offsets::offset_to_position;

/// The API is modelled on an LSP [completion
/// request](https://github.com/microsoft/language-server-protocol/blob/gh-pages/_specifications/specification-3-16.md#textDocument_completion).
/// Input and output are both JSON, the request being a `CompletionParams` object and the response
/// being a `CompletionList` object.
pub fn text_document_completion(schema: String, params: &str) -> String {
    let params = if let Ok(params) = serde_json::from_str::<lsp_types::CompletionParams>(params) {
        params
    } else {
        warn!("Failed to parse params to text_document_completion() as CompletionParams.");
        return serde_json::to_string(&text_document_completion::empty_completion_list()).unwrap();
    };

    let completion_list = text_document_completion::completion(schema, params);

    serde_json::to_string(&completion_list).unwrap()
}

/// This API is modelled on an LSP [code action request](https://github.com/microsoft/language-server-protocol/blob/gh-pages/_specifications/specification-3-16.md#textDocument_codeAction=). Input and output are both JSON, the request being a `CodeActionParams` object and the response being a list of `CodeActionOrCommand` objects.
pub fn code_actions(schema: String, params: &str) -> String {
    let params = if let Ok(params) = serde_json::from_str::<lsp_types::CodeActionParams>(params) {
        params
    } else {
        warn!("Failed to parse params to text_document_completion() as CompletionParams.");
        return serde_json::to_string(&code_actions::empty_code_actions()).unwrap();
    };

    let actions = code_actions::available_actions(schema, params);
    serde_json::to_string(&actions).unwrap()
}

/// The two parameters are:
/// - The Prisma schema to reformat, as a string.
/// - An LSP
/// [DocumentFormattingParams](https://github.com/microsoft/language-server-protocol/blob/gh-pages/_specifications/specification-3-16.md#textDocument_formatting) object, as JSON.
///
/// The function returns the formatted schema, as a string.
///
/// Of the DocumentFormattingParams, we only take into account tabSize, at the moment.
pub fn format(schema: &str, params: &str) -> String {
    let params: lsp_types::DocumentFormattingParams = match serde_json::from_str(params) {
        Ok(params) => params,
        Err(err) => {
            warn!("Error parsing DocumentFormattingParams params: {}", err);
            return schema.to_owned();
        }
    };

    psl::reformat(schema, params.options.tab_size as usize).unwrap_or_else(|| schema.to_owned())
}

pub fn lint(schema: String) -> String {
    lint::run(&schema)
}

/// Function that throws a human-friendly error message when the schema is invalid, following the JSON formatting
/// historically used by the Query Engine's `user_facing_errors::common::SchemaParserError`.
/// When the schema is valid, nothing happens.
/// When the schema is invalid, the function displays a human-friendly error message indicating the schema lines
/// where the errors lie and the total error count, e.g.:
///
/// ```sh
/// The `referentialIntegrity` and `relationMode` attributes cannot be used together. Please use only `relationMode` instead.
///   -->  schema.prisma:5
///   |
/// 4 |   relationMode         = "prisma"
/// 5 |   referentialIntegrity = "foreignKeys"
/// 6 | }
///   |
///
/// Validation Error Count: 1
/// ```
///
/// This function isn't supposed to panic.
pub fn validate(validate_params: String) -> Result<(), String> {
    validate::validate(&validate_params)
}

pub fn native_types(schema: String) -> String {
    native::run(&schema)
}

pub fn preview_features() -> String {
    preview::run()
}

pub fn referential_actions(schema: String) -> String {
    actions::run(&schema)
}

/// This is the same command as get_config()
///
/// Params is a JSON string with the following shape:
///
/// ```ignore
/// interface GetConfigParams {
///   prismaSchema: string
///   ignoreEnvVarErrors?: bool
///   env?: { [key: string]: string }
///   datasourceOverrides?: { [key: string]: string }
/// }
/// ```
/// Params example:
///
/// ```ignore
/// {
///   "prismaSchema": <the prisma schema>,
///   "env": {
///     "DBURL": "postgresql://example.com/mydb"
///   }
/// }
/// ```
///
/// The response is a JSON string with the following shape:
///
/// ```ignore
/// type GetConfigSuccessResponse = any // same as QE getConfig
///
/// interface GetConfigErrorResponse {
///   error: {
///     error_code?: string
///     message: string
///   }
/// }
///
/// type GetConfigResponse = GetConfigErrorResponse | GetConfigSuccessResponse
///
/// ```
pub fn get_config(get_config_params: String) -> Result<String, String> {
    get_config::get_config(&get_config_params)
}

/// This is the same command as get_dmmf()
///
/// Params is a JSON string with the following shape:
///
/// ```ignore
/// interface GetDmmfParams {
///   prismaSchema: string
/// }
/// ```
/// Params example:
///
/// ```ignore
/// {
///   "prismaSchema": <the prisma schema>,
/// }
/// ```
///
/// The response is a JSON string with the following shape:
///
/// ```ignore
/// type GetDmmfSuccessResponse = any // same as QE getDmmf
///
/// interface GetDmmfErrorResponse {
///   error: {
///     error_code?: string
///     message: string
///   }
/// }
///
/// type GetDmmfResponse = GetDmmfErrorResponse | GetDmmfSuccessResponse
///
/// ```
pub fn get_dmmf(get_dmmf_params: String) -> Result<String, String> {
    get_dmmf::get_dmmf(&get_dmmf_params)
}

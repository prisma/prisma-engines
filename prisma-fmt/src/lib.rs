mod actions;
mod get_config;
mod lint;
mod native;
mod preview;
mod text_document_completion;

use log::*;

/// The API is modelled on an LSP [completion
/// request](https://github.com/microsoft/language-server-protocol/blob/gh-pages/_specifications/specification-3-16.md#textDocument_completion).
/// Input and output are both JSON, the request being a `CompletionParams` object and the response
/// being a `CompletionList` object.
pub fn text_document_completion(schema: &str, params: &str) -> String {
    let params = if let Ok(params) = serde_json::from_str::<lsp_types::CompletionParams>(params) {
        params
    } else {
        warn!("Failed to parse params to text_document_completion() as CompletionParams.");
        return serde_json::to_string(&text_document_completion::empty_completion_list()).unwrap();
    };

    let completion_list = text_document_completion::completion(schema, params);

    serde_json::to_string(&completion_list).unwrap()
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
    use datamodel::ast::reformat::Reformatter;

    let params: lsp_types::DocumentFormattingParams = match serde_json::from_str(params) {
        Ok(params) => params,
        Err(err) => {
            warn!("Error parsing DocumentFormattingParams params: {}", err);
            return schema.to_owned();
        }
    };

    let mut out = Vec::with_capacity(schema.len() / 2);
    Reformatter::new(schema).reformat_to(&mut out, params.options.tab_size as usize);
    String::from_utf8_lossy(&out).into_owned()
}

pub fn lint(schema: String) -> String {
    lint::run(&schema)
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
pub fn get_config(get_config_params: String) -> String {
    get_config::get_config(&get_config_params)
}

mod actions;
mod code_actions;
mod get_config;
mod get_dmmf;
mod lint;
mod merge_schemas;
mod native;
mod offsets;
mod preview;
mod schema_file_input;
mod text_document_completion;
mod validate;

use log::*;
pub use offsets::span_to_range;
use psl::{
    datamodel_connector::Connector, diagnostics::FileId, parser_database::ParserDatabase, Configuration, Datasource,
    Generator,
};
use schema_file_input::SchemaFileInput;

#[derive(Debug, Clone, Copy)]
pub(crate) struct LSPContext<'a, T> {
    pub(crate) db: &'a ParserDatabase,
    pub(crate) config: &'a Configuration,
    pub(crate) initiating_file_id: FileId,
    pub(crate) params: &'a T,
}

impl<'a, T> LSPContext<'a, T> {
    pub(crate) fn initiating_file_source(&self) -> &str {
        self.db.source(self.initiating_file_id)
    }

    pub(crate) fn initiating_file_uri(&self) -> &str {
        self.db.file_name(self.initiating_file_id)
    }

    pub(crate) fn datasource(&self) -> Option<&Datasource> {
        self.config.datasources.first()
    }

    pub(crate) fn connector(&self) -> &'static dyn Connector {
        self.datasource()
            .map(|ds| ds.active_connector)
            .unwrap_or(&psl::datamodel_connector::EmptyDatamodelConnector)
    }

    pub(crate) fn generator(&self) -> Option<&'a Generator> {
        self.config.generators.first()
    }
}

/// The API is modelled on an LSP [completion
/// request](https://github.com/microsoft/language-server-protocol/blob/gh-pages/_specifications/specification-3-16.md#textDocument_completion).
/// Input and output are both JSON, the request being a `CompletionParams` object and the response
/// being a `CompletionList` object.
pub fn text_document_completion(schema_files: String, params: &str) -> String {
    let params = if let Ok(params) = serde_json::from_str::<lsp_types::CompletionParams>(params) {
        params
    } else {
        warn!("Failed to parse params to text_document_completion() as CompletionParams.");
        return serde_json::to_string(&text_document_completion::empty_completion_list()).unwrap();
    };

    let Ok(input) = serde_json::from_str::<SchemaFileInput>(&schema_files) else {
        warn!("Failed to parse schema file input");
        return serde_json::to_string(&text_document_completion::empty_completion_list()).unwrap();
    };

    let completion_list = text_document_completion::completion(input.into(), params);

    serde_json::to_string(&completion_list).unwrap()
}

/// This API is modelled on an LSP [code action request](https://github.com/microsoft/language-server-protocol/blob/gh-pages/_specifications/specification-3-16.md#textDocument_codeAction=). Input and output are both JSON, the request being a `CodeActionParams` object and the response being a list of `CodeActionOrCommand` objects.
pub fn code_actions(schema_files: String, params: &str) -> String {
    let params = if let Ok(params) = serde_json::from_str::<lsp_types::CodeActionParams>(params) {
        params
    } else {
        warn!("Failed to parse params to text_document_completion() as CompletionParams.");
        return serde_json::to_string(&code_actions::empty_code_actions()).unwrap();
    };

    let Ok(input) = serde_json::from_str::<SchemaFileInput>(&schema_files) else {
        warn!("Failed to parse schema file input");
        return serde_json::to_string(&text_document_completion::empty_completion_list()).unwrap();
    };

    let actions = code_actions::available_actions(input.into(), params);
    serde_json::to_string(&actions).unwrap()
}

/// The two parameters are:
/// - The [`SchemaFileInput`] to reformat, as a string.
/// - An LSP
/// [DocumentFormattingParams](https://github.com/microsoft/language-server-protocol/blob/gh-pages/_specifications/specification-3-16.md#textDocument_formatting) object, as JSON.
///
/// The function returns the formatted schema, as a string.
/// If the schema or any of the provided parameters is invalid, the function returns the original schema.
/// This function never panics.
///
/// Of the DocumentFormattingParams, we only take into account tabSize, at the moment.
pub fn format(datamodel: String, params: &str) -> String {
    let schema: SchemaFileInput = match serde_json::from_str(&datamodel) {
        Ok(params) => params,
        Err(_) => {
            return datamodel;
        }
    };

    let params: lsp_types::DocumentFormattingParams = match serde_json::from_str(params) {
        Ok(params) => params,
        Err(_) => {
            return datamodel;
        }
    };

    let indent_width = params.options.tab_size as usize;

    match schema {
        SchemaFileInput::Single(single) => psl::reformat(&single, indent_width).unwrap_or(datamodel),
        SchemaFileInput::Multiple(multiple) => {
            let result = psl::reformat_multiple(multiple, indent_width);
            serde_json::to_string(&result).unwrap_or(datamodel)
        }
    }
}

pub fn lint(schema: String) -> String {
    let schema: SchemaFileInput = match serde_json::from_str(&schema) {
        Ok(params) => params,
        Err(serde_err) => {
            panic!("Failed to deserialize SchemaFileInput: {serde_err}");
        }
    };
    lint::run(schema)
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

/// Given a list of Prisma schema files (and their locations), returns the merged schema.
/// This is useful for `@prisma/client` generation, where the client needs a single - potentially large - schema,
/// while still allowing the user to split their schema copies into multiple files.
/// Internally, it uses `[validate]`.
pub fn merge_schemas(params: String) -> Result<String, String> {
    merge_schemas::merge_schemas(&params)
}

pub fn native_types(input: String) -> String {
    native::run(&input)
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

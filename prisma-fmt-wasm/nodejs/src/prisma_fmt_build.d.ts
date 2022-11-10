/* tslint:disable */
/* eslint-disable */
/**
* @param {string} schema
* @param {string} params
* @returns {string}
*/
export function format(schema: string, params: string): string;
/**
* Docs: https://prisma.github.io/prisma-engines/doc/prisma_fmt/fn.get_config.html
* @param {string} params
* @returns {string}
*/
export function get_config(params: string): string;
/**
* @param {string} params
* @returns {string}
*/
export function get_dmmf(params: string): string;
/**
* @param {string} input
* @returns {string}
*/
export function lint(input: string): string;
/**
* @param {string} input
* @returns {string}
*/
export function native_types(input: string): string;
/**
* @param {string} input
* @returns {string}
*/
export function referential_actions(input: string): string;
/**
* @returns {string}
*/
export function preview_features(): string;
/**
* The API is modelled on an LSP [completion
* request](https://github.com/microsoft/language-server-protocol/blob/gh-pages/_specifications/specification-3-16.md#textDocument_completion).
* Input and output are both JSON, the request being a `CompletionParams` object and the response
* being a `CompletionList` object.
* @param {string} schema
* @param {string} params
* @returns {string}
*/
export function text_document_completion(schema: string, params: string): string;
/**
* This API is modelled on an LSP [code action
* request](https://github.com/microsoft/language-server-protocol/blob/gh-pages/_specifications/specification-3-16.md#textDocument_codeAction=).
* Input and output are both JSON, the request being a
* `CodeActionParams` object and the response being a list of
* `CodeActionOrCommand` objects.
* @param {string} schema
* @param {string} params
* @returns {string}
*/
export function code_actions(schema: string, params: string): string;
/**
* Trigger a panic inside the wasm module. This is only useful in development for testing panic
* handling.
*/
export function debug_panic(): void;
/**
* A version of `JdbcString` to be used from web-assembly.
*/
export class AdoNetString {
  free(): void;
/**
* A constructor to create a new `AdoNet`, used from JavaScript with
* `new AdoNet("server=tcp:localhost,1433")`.
* @param {string} s
*/
  constructor(s: string);
/**
* Get a parameter from the connection's key-value pairs
* @param {string} key
* @returns {string | undefined}
*/
  get(key: string): string | undefined;
/**
* Set a parameter value to the connection's key-value pairs. If replacing
* a pre-existing value, returns the old value.
* @param {string} key
* @param {string} value
* @returns {string | undefined}
*/
  set(key: string, value: string): string | undefined;
/**
* Get a string representation of the `AdoNetString`.
* @returns {string}
*/
  to_string(): string;
}
/**
* A version of `JdbcString` to be used from web-assembly.
*/
export class JdbcString {
  free(): void;
/**
* A constructor to create a new `JdbcInstance`, used from JavaScript with
* `new JdbcString("sqlserver://...")`.
* @param {string} s
*/
  constructor(s: string);
/**
* Access the connection sub-protocol
* @returns {string}
*/
  sub_protocol(): string;
/**
* Access the connection server name
* @returns {string | undefined}
*/
  server_name(): string | undefined;
/**
* Access the connection's instance name
* @returns {string | undefined}
*/
  instance_name(): string | undefined;
/**
* Access the connection's port
* @returns {number | undefined}
*/
  port(): number | undefined;
/**
* Get a parameter from the connection's key-value pairs
* @param {string} key
* @returns {string | undefined}
*/
  get(key: string): string | undefined;
/**
* Set a parameter value to the connection's key-value pairs. If replacing
* a pre-existing value, returns the old value.
* @param {string} key
* @param {string} value
* @returns {string | undefined}
*/
  set(key: string, value: string): string | undefined;
/**
* Get a string representation of the `JdbcString`.
* @returns {string}
*/
  to_string(): string;
}

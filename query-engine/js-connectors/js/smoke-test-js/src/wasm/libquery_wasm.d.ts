/* tslint:disable */
/* eslint-disable */
/**
* @returns {any}
*/
export function version(): any;
/**
* @param {string} datamodel_string
* @returns {string}
*/
export function dmmf(datamodel_string: string): string;
/**
* @param {string | undefined} panic_message
*/
export function debug_panic(panic_message?: string): void;
/**
*/
export function initPanicHook(): void;
/**
* Proxy is a struct wrapping a javascript object that exhibits basic primitives for
* querying and executing SQL (i.e. a client connector). The Proxy uses sys::Function to
* invoke the code within the node runtime that implements the client connector.
*/
export class Proxy {
  free(): void;
/**
* @param {object} js_connector
*/
  constructor(js_connector: object);
}
/**
* The main query engine used by JS
*/
export class QueryEngine {
  free(): void;
/**
* Parse a validated datamodel and configuration to allow connecting later on.
* @param {any} options
* @param {Function} callback
* @param {Proxy | undefined} maybe_driver
*/
  constructor(options: any, callback: Function, maybe_driver?: Proxy);
/**
* Connect to the database, allow queries to be run.
* @param {string} trace
* @returns {Promise<void>}
*/
  connect(trace: string): Promise<void>;
/**
* Disconnect and drop the core. Can be reconnected later with `#connect`.
* @param {string} trace
* @returns {Promise<void>}
*/
  disconnect(trace: string): Promise<void>;
/**
* If connected, sends a query to the core and returns the response.
* @param {string} body
* @param {string} trace
* @param {string | undefined} tx_id
* @returns {Promise<string>}
*/
  query(body: string, trace: string, tx_id?: string): Promise<string>;
/**
* If connected, attempts to start a transaction in the core and returns its ID.
* @param {string} input
* @param {string} trace
* @returns {Promise<string>}
*/
  startTransaction(input: string, trace: string): Promise<string>;
/**
* If connected, attempts to commit a transaction with id `tx_id` in the core.
* @param {string} tx_id
* @param {string} _trace
* @returns {Promise<string>}
*/
  commitTransaction(tx_id: string, _trace: string): Promise<string>;
/**
* @param {string} trace
* @returns {Promise<string>}
*/
  dmmf(trace: string): Promise<string>;
/**
* If connected, attempts to roll back a transaction with id `tx_id` in the core.
* @param {string} tx_id
* @param {string} _trace
* @returns {Promise<string>}
*/
  rollbackTransaction(tx_id: string, _trace: string): Promise<string>;
/**
* Loads the query schema. Only available when connected.
* @returns {Promise<string>}
*/
  sdlSchema(): Promise<string>;
}

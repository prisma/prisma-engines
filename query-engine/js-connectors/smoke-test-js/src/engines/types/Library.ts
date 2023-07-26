import type { QueryEngineConfig } from './QueryEngine.js'

export type QueryEngineInstance = {
  connect(headers: string): Promise<void>
  disconnect(headers: string): Promise<void>
  /**
   * @param requestStr JSON.stringified `QueryEngineRequest | QueryEngineBatchRequest`
   * @param headersStr JSON.stringified `QueryEngineRequestHeaders`
   */
  query(requestStr: string, headersStr: string, transactionId?: string): Promise<string>
  sdlSchema(): Promise<string>
  dmmf(traceparent: string): Promise<string>
  startTransaction(options: string, traceHeaders: string): Promise<string>
  commitTransaction(id: string, traceHeaders: string): Promise<string>
  rollbackTransaction(id: string, traceHeaders: string): Promise<string>
  metrics(options: string): Promise<string>
}

export interface ResultSet {
  columnTypes: Array<ColumnType>
  columnNames: Array<string>
  rows: Array<Array<any>>
}

export interface Query {
  sql: string
  args: Array<any>
}

// Same order as in rust js-connectors' `ColumnType`
export const enum ColumnType {
  Int32,
  Int64,
  Float,
  Double,
  Numeric,
  Boolean,
  Char,
  Text,
  Date,
  Time,
  DateTime,
  Json,
  Enum,
  Bytes,
  // Set,
  // Array,
  // ...
}

export type Connector = {
  queryRaw: (params: Query) => Promise<ResultSet>
  executeRaw: (params: Query) => Promise<number>
  version: () => Promise<string | undefined>
  isHealthy: () => boolean
  flavor: string,
}

export type Closeable = {
  close: () => Promise<void>
}

export interface QueryEngineConstructor {
  new(config: QueryEngineConfig, logger: (log: string) => void, nodejsFnCtx?: Connector): QueryEngineInstance
}

export interface LibraryLoader {
  loadLibrary(): Promise<Library>
}

// Main
export type Library = {
  QueryEngine: QueryEngineConstructor

  version: () => {
    // The commit hash of the engine
    commit: string
    // Currently 0.1.0 (Set in Cargo.toml)
    version: string
  }
  /**
   * This returns a string representation of `DMMF.Document`
   */
  dmmf: (datamodel: string) => Promise<string>
}

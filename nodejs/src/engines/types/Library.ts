import type { QueryEngineConfig } from './QueryEngine'

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

export type NodejsFunctionContext = {
  queryRaw: () => Promise<unknown>
  queryRawTyped: () => Promise<unknown>
  executeRaw: () => Promise<unknown>
  executeRawTyped: () => Promise<unknown>
  version: () => string
}

export interface QueryEngineConstructor {
  new (config: QueryEngineConfig, logger: (log: string) => void): QueryEngineInstance
}

export interface QueryEngineNodeDriversConstructor {
  new (config: QueryEngineConfig, logger: (log: string) => void, nodejsFnCtx: NodejsFunctionContext): QueryEngineInstance
}

export interface LibraryLoader {
  loadLibrary(): Promise<Library>
}

// Main
export type Library = {
  QueryEngine: QueryEngineConstructor
  QueryEngineNodeDrivers: QueryEngineNodeDriversConstructor

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

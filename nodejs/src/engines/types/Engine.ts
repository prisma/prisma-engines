import * as Transaction from './Transaction'
import { TracingHelper } from '../TracingHelper'
import { EngineProtocol, QueryEngineResult } from './QueryEngine'
import { NodejsFunctionContext } from './Library'

// EventEmitter represents a platform-agnostic slice of NodeJS.EventEmitter,
export interface EventEmitter {
  on(event: string, listener: (...args: any[]) => void): unknown
  emit(event: string, args?: any): boolean
}

export interface EnvValue {
  fromEnvVar: null | string
  value: null | string
}

export interface DatasourceOverwrite {
  name: string
  url?: string
  env?: string
}

export type ConnectorType =
  | 'mysql'
  | 'mongodb'
  | 'sqlite'
  | 'postgresql'
  | 'postgres' // TODO: we could normalize postgres to postgresql this in engines to reduce the complexity?
  | 'sqlserver'
  | 'cockroachdb'

  // TODO: this should be removed in favor of `'sqlserver'`, as per `getConfig({ ... }).datasources[0]?.provider` from a schema with `provider = "sqlserver"`
  // 'jdbc:sqlserver' has been removed in https://github.com/prisma/prisma-engines/pull/2830
  | 'jdbc:sqlserver'

export interface DataSource {
  name: string
  provider: ConnectorType
  activeProvider: ConnectorType
  url: EnvValue
  directUrl?: EnvValue
  schemas: string[] | []
}

export interface EngineConfig {
  nodejsFnCtx: NodejsFunctionContext
  cwd: string
  dirname?: string
  datamodelPath: string
  enableDebugLogs?: boolean
  allowTriggerPanic?: boolean // dangerous! https://github.com/prisma/prisma-engines/issues/764
  prismaPath?: string
  // generator?: GeneratorConfig
  datasources?: DatasourceOverwrite[]
  showColors?: boolean
  logQueries?: boolean
  logLevel?: 'info' | 'warn'
  env: Record<string, string>
  flags?: string[]
  clientVersion?: string
  previewFeatures?: string[]
  engineEndpoint?: string
  activeProvider?: string
  logEmitter: EventEmitter
  engineProtocol: EngineProtocol

  /**
   * The string hash that was produced for a given schema
   * @remarks only used for the purpose of data proxy
   */
  inlineSchemaHash?: string

  /**
   * The helper for interaction with OTEL tracing
   * @remarks enabling is determined by the client and @prisma/instrumentation package
   */
  tracingHelper: TracingHelper

  /**
   * Information about whether we have not found a schema.prisma file in the
   * default location, and that we fell back to finding the schema.prisma file
   * in the current working directory. This usually means it has been bundled.
   */
  isBundled?: boolean
}

export type EngineEventType = 'query' | 'info' | 'warn' | 'error' | 'beforeExit'

export type RequestOptions<InteractiveTransactionPayload> = {
  traceparent?: string
  numTry?: number
  interactiveTransaction?: InteractiveTransactionOptions<InteractiveTransactionPayload>
  isWrite: boolean
}

export type RequestBatchOptions<InteractiveTransactionPayload> = {
  transaction?: TransactionOptions<InteractiveTransactionPayload>
  traceparent?: string
  numTry?: number
  containsWrite: boolean
}

export type InteractiveTransactionOptions<Payload> = Transaction.InteractiveTransactionInfo<Payload>

export type TransactionOptions<InteractiveTransactionPayload> =
  | {
      kind: 'itx'
      options: InteractiveTransactionOptions<InteractiveTransactionPayload>
    }
  | {
      kind: 'batch'
      options: BatchTransactionOptions
    }

export type BatchTransactionOptions = {
  isolationLevel?: Transaction.IsolationLevel
}

export type BatchQueryEngineResult<T> = QueryEngineResult<T> | Error

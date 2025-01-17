import * as S from '@effect/schema/Schema'

const SchemaId = S.number.pipe(S.int(), S.nonNegative()).pipe(S.brand('SchemaId'))
export type SchemaId = S.Schema.Type<typeof SchemaId>

const InitializeSchemaParams = S.struct({
  schemaId: SchemaId,
  schema: S.string,
  url: S.string,
  migrationScript: S.optional(S.string),
})
export type InitializeSchemaParams = S.Schema.Type<typeof InitializeSchemaParams>

const InitializeSchema = S.struct({
  method: S.literal('initializeSchema'),
  params: InitializeSchemaParams,
})

const QueryParams = S.struct({
  schemaId: SchemaId,
  query: S.record(S.string, S.unknown),
  txId: S.nullable(S.string),
})
export type QueryParams = S.Schema.Type<typeof QueryParams>

const Query = S.struct({
  method: S.literal('query'),
  params: QueryParams,
})

const StartTxParams = S.struct({
  schemaId: SchemaId,
  options: S.unknown,
})
export type StartTxParams = S.Schema.Type<typeof StartTxParams>

const StartTx = S.struct({
  method: S.literal('startTx'),
  params: StartTxParams,
})

const CommitTxParams = S.struct({
  schemaId: SchemaId,
  txId: S.string,
})
export type CommitTxParams = S.Schema.Type<typeof CommitTxParams>

const CommitTx = S.struct({
  method: S.literal('commitTx'),
  params: CommitTxParams,
})

const RollbackTxParams = S.struct({
  schemaId: SchemaId,
  txId: S.string,
})
export type RollbackTxParams = S.Schema.Type<typeof RollbackTxParams>

const RollbackTx = S.struct({
  method: S.literal('rollbackTx'),
  params: RollbackTxParams,
})

const TeardownParams = S.struct({
  schemaId: SchemaId,
})
export type TeardownParams = S.Schema.Type<typeof TeardownParams>

const TeardownSchema = S.struct({
  method: S.literal('teardown'),
  params: TeardownParams,
})

const GetLogsParams = S.struct({
  schemaId: SchemaId,
})
export type GetLogsParams = S.Schema.Type<typeof GetLogsParams>

const GetLogs = S.struct({
  method: S.literal('getLogs'),
  params: GetLogsParams,
})

export const Request = S.extend(
  S.struct({
    jsonrpc: S.literal('2.0'),
    id: S.number.pipe(S.int()),
  }),
  S.union(
    InitializeSchema,
    Query,
    StartTx,
    CommitTx,
    RollbackTx,
    TeardownSchema,
    GetLogs,
  ),
)

export type Request = S.Schema.Type<typeof Request>

export const RequestFromString = S.transform(
  S.string,
  Request,
  (str) => JSON.parse(str),
  (request) => JSON.stringify(request),
)
export type RequestFromString = S.Schema.Type<typeof RequestFromString>

export type Response = OkResponse | ErrResponse

export interface OkResponse {
    jsonrpc: '2.0'
    result: unknown
    error?: never
    id: number
}

export interface ErrResponse {
    jsonrpc: '2.0'
    error: RpcError
    result?: never
    id: number
}

export interface RpcError {
    code: number
    message: string
    data?: unknown
}

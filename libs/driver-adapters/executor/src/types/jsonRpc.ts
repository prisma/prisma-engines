import * as S from '@effect/schema/Schema'

const SchemaId = S.number
  .pipe(S.int(), S.nonNegative())
  .pipe(S.brand('SchemaId'))
export type SchemaId = S.Schema.Type<typeof SchemaId>

export const InitializeSchemaParams = S.struct({
  schemaId: SchemaId,
  schema: S.string,
  url: S.string,
  migrationScript: S.optional(S.string),
})
export type InitializeSchemaParams = S.Schema.Type<
  typeof InitializeSchemaParams
>

const InitializeSchema = S.struct({
  method: S.literal('initializeSchema'),
  params: InitializeSchemaParams,
})

const JsonProtocolQuery = S.struct({
  modelName: S.optional(S.nullable(S.string)),
  action: S.union(
    S.literal('findUnique'),
    S.literal('findUniqueOrThrow'),
    S.literal('findFirst'),
    S.literal('findFirstOrThrow'),
    S.literal('findMany'),
    S.literal('createOne'),
    S.literal('createMany'),
    S.literal('createManyAndReturn'),
    S.literal('updateOne'),
    S.literal('updateMany'),
    S.literal('updateManyAndReturn'),
    S.literal('deleteOne'),
    S.literal('deleteMany'),
    S.literal('upsertOne'),
    S.literal('aggregate'),
    S.literal('groupBy'),
    S.literal('executeRaw'),
    S.literal('queryRaw'),
    S.literal('runCommandRaw'),
    S.literal('findRaw'),
    S.literal('aggregateRaw'),
  ),
  query: S.record(S.string, S.unknown),
})
export type JsonProtocolQuery = S.Schema.Type<typeof JsonProtocolQuery>

const JsonProtocolBatchQuery = S.struct({
  batch: S.array(JsonProtocolQuery),
  transaction: S.optional(
    S.nullable(
      S.struct({
        isolationLevel: S.optional(S.nullable(S.string)),
      }),
    ),
  ),
})

export const QueryParams = S.struct({
  schemaId: SchemaId,
  query: S.union(JsonProtocolQuery, JsonProtocolBatchQuery),
  txId: S.nullable(S.string),
})
export type QueryParams = S.Schema.Type<typeof QueryParams>

const Query = S.struct({
  method: S.literal('query'),
  params: QueryParams,
})

export const TxOptions = S.struct({
  max_wait: S.number.pipe(S.int(), S.nonNegative()),
  timeout: S.number.pipe(S.int(), S.nonNegative()),
  isolation_level: S.optional(S.nullable(S.string)),
})
export type TxOptions = S.Schema.Type<typeof TxOptions>

export const StartTxParams = S.struct({
  schemaId: SchemaId,
  options: TxOptions,
})
export type StartTxParams = S.Schema.Type<typeof StartTxParams>

const StartTx = S.struct({
  method: S.literal('startTx'),
  params: StartTxParams,
})

export const CommitTxParams = S.struct({
  schemaId: SchemaId,
  txId: S.string,
})
export type CommitTxParams = S.Schema.Type<typeof CommitTxParams>

const CommitTx = S.struct({
  method: S.literal('commitTx'),
  params: CommitTxParams,
})

export const RollbackTxParams = S.struct({
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

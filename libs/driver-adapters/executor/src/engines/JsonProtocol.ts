import type { JsonTaggedValue } from '@prisma/client-engine-runtime'
import * as Transaction from './Transaction.js'

export type JsonQuery = {
  modelName?: string
  action: JsonQueryAction
  query: JsonFieldSelection
}

export type JsonBatchQuery = {
  batch: JsonQuery[]
  transaction?: { isolationLevel?: Transaction.IsolationLevel }
}

export type JsonQueryAction =
  | 'findUnique'
  | 'findUniqueOrThrow'
  | 'findFirst'
  | 'findFirstOrThrow'
  | 'findMany'
  | 'createOne'
  | 'createMany'
  | 'updateOne'
  | 'updateMany'
  | 'deleteOne'
  | 'deleteMany'
  | 'upsertOne'
  | 'aggregate'
  | 'groupBy'
  | 'executeRaw'
  | 'queryRaw'
  | 'runCommandRaw'
  | 'findRaw'
  | 'aggregateRaw'

export type JsonFieldSelection = {
  arguments?: Record<string, JsonArgumentValue>
  selection: JsonSelectionSet
}

export type JsonSelectionSet = {
  $scalars?: boolean
  $composites?: boolean
} & {
  [fieldName: string]: boolean | JsonFieldSelection
}

export type JsonArgumentValue =
  | number
  | string
  | boolean
  | null
  | JsonTaggedValue
  | JsonArgumentValue[]
  | { [key: string]: JsonArgumentValue }

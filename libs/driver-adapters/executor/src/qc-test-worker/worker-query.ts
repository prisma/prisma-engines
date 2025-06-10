import * as util from 'node:util'

import {
  noopTracingHelper,
  QueryEvent,
  QueryInterpreter,
  type QueryInterpreterTransactionManager,
  QueryPlanNode,
  safeJsonStringify,
  type TransactionManager,
  UserFacingError,
} from '@prisma/client-engine-runtime'
import {
  ColumnType,
  ColumnTypeEnum,
  IsolationLevel,
  SqlQueryable,
  SqlResultSet,
} from '@prisma/driver-adapter-utils'
import Decimal from 'decimal.js'

import { JsonOutputTaggedValue } from '../engines/JsonProtocol'
import { withLocalPanicHandler } from '../panic'
import { QueryCompiler } from '../query-compiler'
import { JsonProtocolQuery, QueryParams } from '../types/jsonRpc'
import { assertNever, debug } from '../utils'
import type { State } from './worker'
import { parseIsolationLevel } from './worker-transaction'

export function query(
  params: QueryParams,
  state: State,
  logs: string[],
): Promise<string> {
  const pipeline = new QueryPipeline(state, logs)
  return pipeline.run(params.query, params.txId)
}

class QueryPipeline {
  private compiler: QueryCompiler
  private driverAdapter: SqlQueryable
  private transactionManager: TransactionManager

  constructor(
    state: State,
    private logs: string[],
  ) {
    this.compiler = state.compiler
    this.driverAdapter = state.driverAdapter
    this.transactionManager = state.transactionManager
  }

  async run(query: QueryParams['query'], txId: QueryParams['txId']) {
    try {
      if ('batch' in query) {
        const { batch, transaction } = query

        // A transactional batch starts its own transaction, and hence doesn't
        // need the transaction ID, as we don't currently support nested
        // transactions. An independent batch, however, may itself be executed
        // within an interactive transaction, and therefore needs the current
        // transaction ID.
        const results = transaction
          ? await this.executeTransactionalBatch(
              batch,
              parseIsolationLevel(transaction.isolationLevel),
            )
          : await this.executeIndependentBatch(batch, txId)

        debug('🟢 Batch query results: ', results)

        return safeJsonStringify({
          batchResult: batch.map((query, index) =>
            getResponseInQeFormat(query, results[index]),
          ),
        })
      } else {
        const queryable = txId
          ? this.transactionManager.getTransaction({ id: txId }, 'query')
          : this.driverAdapter

        if (!queryable) {
          throw new Error(
            `No transaction with id ${txId} found. Please call 'startTx' first.`,
          )
        }

        const result = await this.executeQuery(queryable, query, !txId)

        debug('🟢 Query result: ', util.inspect(result, false, null, true))

        return safeJsonStringify(getResponseInQeFormat(query, result))
      }
    } catch (error) {
      if (error instanceof UserFacingError) {
        return safeJsonStringify({
          errors: [error.toQueryResponseErrorObject()],
        })
      }
      throw error
    }
  }

  private async executeQuery(
    queryable: SqlQueryable,
    query: JsonProtocolQuery,
    allowTransaction: boolean,
  ) {
    let queryPlan: QueryPlanNode
    try {
      queryPlan = withLocalPanicHandler(() =>
        this.compiler.compile(safeJsonStringify(query)),
      )
    } catch (error) {
      if (typeof error.message === 'string' && typeof error.code === 'string') {
        throw new UserFacingError(error.message, error.code, error.meta)
      } else {
        throw error
      }
    }

    debug('🟢 Query plan: ', util.inspect(queryPlan, false, null, true))

    const qiTransactionManager = (
      allowTransaction
        ? { enabled: true, manager: this.transactionManager }
        : { enabled: false }
    ) satisfies QueryInterpreterTransactionManager

    const interpreterOpts = {
      transactionManager: qiTransactionManager,
      placeholderValues: {},
      onQuery: (event: QueryEvent) => {
        this.logs.push(safeJsonStringify(event))
      },
      tracingHelper: noopTracingHelper,
    }

    const interpreter = QueryInterpreter.forSql(interpreterOpts)

    return interpreter.run(queryPlan, queryable)
  }

  private async executeIndependentBatch(
    queries: readonly JsonProtocolQuery[],
    txId: QueryParams['txId'],
  ) {
    const queryable =
      txId !== null
        ? this.transactionManager.getTransaction({ id: txId }, 'batch query')
        : this.driverAdapter

    const canStartNewTransaction = txId === null

    return Promise.all(
      queries.map((query) =>
        this.executeQuery(queryable, query, canStartNewTransaction),
      ),
    )
  }

  private async executeTransactionalBatch(
    queries: readonly JsonProtocolQuery[],
    isolationLevel?: IsolationLevel,
  ) {
    const txInfo = await this.transactionManager.startTransaction({
      maxWait: 2000,
      timeout: 5000,
      isolationLevel,
    })

    const transaction = this.transactionManager.getTransaction(
      txInfo,
      'batch query',
    )

    try {
      const results: unknown[] = []
      for (const query of queries) {
        const result = await this.executeQuery(transaction, query, false)
        results.push(result)
      }

      await this.transactionManager.commitTransaction(txInfo.id)

      return results
    } catch (err) {
      await this.transactionManager
        .rollbackTransaction(txInfo.id)
        .catch(console.error)
      throw err
    }
  }
}

function getResponseInQeFormat(query: JsonProtocolQuery, result: unknown) {
  return {
    data: {
      [getFullOperationName(query)]: normalizeJsonProtocolValues(result),
    },
  }
}

function getFullOperationName(query: JsonProtocolQuery): string {
  switch (query.action) {
    case 'createManyAndReturn':
      return `createMany${query.modelName}AndReturn`
    case 'updateManyAndReturn':
      return `updateMany${query.modelName}AndReturn`
    case 'findFirstOrThrow':
      return `findFirst${query.modelName}OrThrow`
    case 'findUniqueOrThrow':
      return `findUnique${query.modelName}OrThrow`
    default:
      if (query.modelName) {
        return query.action + query.modelName
      } else {
        return query.action
      }
  }
}

function normalizeJsonProtocolValues(result: unknown): unknown {
  if (result === null) {
    return result
  }

  if (Array.isArray(result)) {
    return result.map(normalizeJsonProtocolValues)
  }

  if (typeof result === 'object') {
    if (isTaggedValue(result)) {
      return normalizeTaggedValue(result)
    }

    // avoid mapping class instances
    if (result.constructor !== null && result.constructor.name !== 'Object') {
      return result
    }

    return mapObjectValues(result, normalizeJsonProtocolValues)
  }

  return result
}

function isTaggedValue(value: unknown): value is JsonOutputTaggedValue {
  return (
    value !== null &&
    typeof value == 'object' &&
    typeof value['$type'] === 'string'
  )
}

/**
 * Normalizes the value inside a tagged value to match the snapshots in tests.
 * Sometimes there are multiple equally valid representations of the same value
 * (e.g. a decimal string may contain an arbitrary number of trailing zeros,
 * datetime strings may specify the UTC offset as either '+00:00' or 'Z', etc).
 * Since these differences have no effect on the actual values received from the
 * Prisma Client once the response is deserialized to JavaScript values, we don't
 * spend extra CPU cycles on normalizing them in the data mapper. Instead, we
 * patch and normalize them here to ensure they are consistent with the snapshots
 * in the query engine tests.
 */
function normalizeTaggedValue({
  $type,
  value,
}: JsonOutputTaggedValue): JsonOutputTaggedValue {
  switch ($type) {
    case 'BigInt':
      return { $type, value: String(value) }
    case 'Bytes':
      return { $type, value }
    case 'DateTime':
      return { $type, value: new Date(value).toISOString() }
    case 'Decimal':
      return { $type, value: String(new Decimal(value)) }
    case 'Json':
      return { $type, value: JSON.stringify(JSON.parse(value)) }
    default:
      assertNever(value, 'Unknown tagged value')
  }
}

function mapObjectValues<K extends PropertyKey, T, U>(
  object: Record<K, T>,
  mapper: (value: T, key: K) => U,
): Record<K, U> {
  const result = {} as Record<K, U>

  for (const key of Object.keys(object)) {
    result[key] = mapper(object[key] as T, key as K)
  }

  return result
}

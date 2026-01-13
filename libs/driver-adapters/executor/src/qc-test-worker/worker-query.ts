import * as util from 'node:util'

import {
  BatchResponse,
  convertCompactedRows,
  DataMapperError,
  noopTracingHelper,
  normalizeJsonProtocolValues,
  normalizeRawJsonProtocolResponse,
  QueryEvent,
  QueryInterpreter,
  type QueryInterpreterTransactionManager,
  QueryPlanNode,
  RawResponse,
  type TransactionManager,
  UserFacingError,
} from '@prisma/client-engine-runtime'
import {
  IsolationLevel,
  SqlDriverAdapter,
  SqlQueryable,
} from '@prisma/driver-adapter-utils'

import { withLocalPanicHandler } from '../panic.js'
import { QueryCompiler } from '../query-compiler.js'
import { JsonProtocolQuery, QueryParams } from '../types/jsonRpc.js'
import { debug } from '../utils.js'
import type { State } from './worker.js'
import { parseIsolationLevel } from './worker-transaction.js'

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
  private driverAdapter: SqlDriverAdapter
  private interpreter: QueryInterpreter
  private transactionManager: TransactionManager

  constructor(
    state: State,
    private logs: string[],
  ) {
    this.compiler = state.compiler
    this.driverAdapter = state.driverAdapter
    this.transactionManager = state.transactionManager
    this.interpreter = QueryInterpreter.forSql({
      onQuery: (event) => {
        this.logs.push(safeJsonStringify(event))
      },
      tracingHelper: noopTracingHelper,
      provider: this.driverAdapter.provider,
      connectionInfo: this.driverAdapter.getConnectionInfo?.(),
    })
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

        debug(
          '🟢 Batch query results: ',
          util.inspect(results, false, null, true),
        )

        return safeJsonStringify({
          batchResult: batch.map((query, index) =>
            getResponseInQeFormat(query, results[index]),
          ),
        })
      } else {
        const queryable = txId
          ? await this.transactionManager.getTransaction({ id: txId }, 'query')
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
      } else if (error instanceof DataMapperError) {
        return safeJsonStringify({
          errors: [
            {
              error: error.message,
              user_facing_error: {
                is_panic: false,
                message: error.message,
              },
            },
          ],
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

    return this.#executeQueryPlan(queryable, queryPlan, allowTransaction)
  }

  async #executeQueryPlan(
    queryable: SqlQueryable,
    queryPlan: QueryPlanNode,
    allowTransaction: boolean,
  ) {
    const qiTransactionManager = (
      allowTransaction
        ? { enabled: true, manager: this.transactionManager }
        : { enabled: false }
    ) satisfies QueryInterpreterTransactionManager

    return this.interpreter.run(queryPlan, {
      queryable,
      transactionManager: qiTransactionManager,
      scope: {},
    })
  }

  private async executeIndependentBatch(
    queries: readonly JsonProtocolQuery[],
    txId: QueryParams['txId'],
  ) {
    const queryable =
      txId !== null
        ? await this.transactionManager.getTransaction(
            { id: txId },
            'batch query',
          )
        : this.driverAdapter

    const canStartNewTransaction = txId === null

    return await this.#executeBatchOn(
      queryable,
      queries,
      canStartNewTransaction,
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

    const transaction = await this.transactionManager.getTransaction(
      txInfo,
      'batch query',
    )

    try {
      const results = await this.#executeBatchOn(transaction, queries, false)
      await this.transactionManager.commitTransaction(txInfo.id)
      return results
    } catch (err) {
      await this.transactionManager
        .rollbackTransaction(txInfo.id)
        .catch(console.error)
      throw err
    }
  }

  async #executeBatchOn(
    queryable: SqlQueryable,
    queries: readonly JsonProtocolQuery[],
    canStartNewTransaction: boolean,
  ): Promise<unknown[]> {
    let compiledBatch: BatchResponse
    try {
      compiledBatch = withLocalPanicHandler(() =>
        this.compiler.compileBatch(safeJsonStringify({ batch: queries })),
      )
    } catch (error) {
      if (typeof error.message === 'string' && typeof error.code === 'string') {
        throw new UserFacingError(error.message, error.code, error.meta)
      } else {
        throw error
      }
    }

    debug(
      '🟢 Batch query plan: ',
      util.inspect(compiledBatch, false, null, true),
    )

    const results: unknown[] = []

    switch (compiledBatch.type) {
      case 'multi':
        for (const plan of compiledBatch.plans) {
          results.push(
            await this.#executeQueryPlan(
              queryable,
              plan,
              canStartNewTransaction,
            ),
          )
        }
        break

      case 'compacted': {
        if (!queries.every((q) => q.action === queries[0].action)) {
          throw new Error('All queries in a batch must have the same action')
        }

        const rows = await this.#executeQueryPlan(
          queryable,
          compiledBatch.plan,
          canStartNewTransaction,
        )

        results.push(...convertCompactedRows(rows as {}[], compiledBatch))
      }
    }

    return results
  }
}

function getResponseInQeFormat(query: JsonProtocolQuery, result: unknown) {
  return {
    data: {
      [getFullOperationName(query)]:
        query.action === 'queryRaw'
          ? normalizeRawJsonProtocolResponse(result as RawResponse)
          : normalizeJsonProtocolValues(result),
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

/**
 * `JSON.stringify` wrapper with custom replacer function that handles nested
 * BigInt and Buffer values.
 */
export function safeJsonStringify(obj: unknown): string {
  // Recursively convert any values that we cannot pass
  // to JSON.stringify to a serializable format.
  // We cannot use the JSON.serialize(obj, transform), because it calls
  // `toJSON` methods on objects, which we want to avoid.
  function transformValue(value: unknown): unknown {
    if (typeof value === 'bigint') {
      return value.toString()
    } else if (ArrayBuffer.isView(value)) {
      return Buffer.from(
        value.buffer,
        value.byteOffset,
        value.byteLength,
      ).toString('base64')
    } else if (Array.isArray(value)) {
      return value.map(transformValue)
    } else if (value !== null && typeof value === 'object') {
      const result: Record<string, unknown> = {}
      for (const [key, val] of Object.entries(value)) {
        result[key] = transformValue(val)
      }
      return result
    }
    return value
  }

  return JSON.stringify(transformValue(obj))
}

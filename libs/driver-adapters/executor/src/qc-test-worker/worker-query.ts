import * as util from 'node:util'

import {
  noopTracingHelper,
  normalizeJsonProtocolValues,
  QueryEvent,
  QueryInterpreter,
  type QueryInterpreterTransactionManager,
  QueryPlanNode,
  safeJsonStringify,
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
      provider: this.driverAdapter.provider,
      connectionInfo: this.driverAdapter.getConnectionInfo?.(),
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

import * as util from 'node:util'
import {
  SqlQueryable,
  IsolationLevel,
} from '@prisma/driver-adapter-utils'
import { JsonProtocolQuery, QueryParams } from '../types/jsonRpc'
import type { State } from './worker'
import { debug } from '../utils'
import {
  QueryInterpreter,
  type QueryInterpreterTransactionManager,
  type TransactionManager,
} from '@prisma/client-engine-runtime'
import { QueryCompiler } from '../query-compiler'
import { parseIsolationLevel } from './worker-transaction'
import { withLocalPanicHandler } from '../panic'

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
    if ('batch' in query) {
      const { batch, transaction } = query

      const results = transaction
        ? await this.executeTransactionalBatch(
          batch,
          parseIsolationLevel(transaction.isolationLevel),
        )
        : await this.executeIndependentBatch(batch)

      debug('🟢 Batch query results: ', results)

      return JSON.stringify({
        batchResult: batch.map((query, index) =>
          getResponseInQeFormat(query, results[index]),
        ),
      })
    } else {
      const queryable = txId
        ? this.transactionManager.getTransaction({ id: txId }, query.action)
        : this.driverAdapter

      if (!queryable) {
        throw new Error(
          `No transaction with id ${txId} found. Please call 'startTx' first.`,
        )
      }

      const result = await this.executeQuery(queryable, query, !txId)

      debug('🟢 Query result: ', result)

      return JSON.stringify(getResponseInQeFormat(query, result))
    }
  }

  private async executeQuery(
    queryable: SqlQueryable,
    query: JsonProtocolQuery,
    allowTransaction: boolean,
  ) {
    const queryPlanString = withLocalPanicHandler(() =>
      this.compiler.compile(JSON.stringify(query)),
    )

    const queryPlan = JSON.parse(queryPlanString)

    debug('🟢 Query plan: ', util.inspect(queryPlan, false, null, true))

    const qiTransactionManager = (
      allowTransaction ? { enabled: true, manager: this.transactionManager } : { enabled: false }
    ) satisfies QueryInterpreterTransactionManager

    const interpreter = new QueryInterpreter({
      transactionManager: qiTransactionManager,
      placeholderValues: {},
      onQuery: (event) => {
        this.logs.push(JSON.stringify(event))
      },
    })

    return interpreter.run(queryPlan, queryable)
  }

  private async executeIndependentBatch(queries: readonly JsonProtocolQuery[]) {
    return Promise.all(
      queries.map((query) => this.executeQuery(this.driverAdapter, query, true)),
    )
  }

  private async executeTransactionalBatch(
    queries: readonly JsonProtocolQuery[],
    isolationLevel?: IsolationLevel,
  ) {
    const txInfo = await this.transactionManager.startTransaction({
      isolationLevel,
    })

    const transaction = this.transactionManager.getTransaction(
      txInfo,
      'batch transaction query',
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
      [getFullOperationName(query)]: getOperationResultInQeFormat(result),
    },
  }
}

function getFullOperationName(query: JsonProtocolQuery): string {
  if (query.modelName) {
    return query.action + query.modelName
  } else {
    return query.action
  }
}

function getOperationResultInQeFormat(result: unknown) {
  if (typeof result === 'number') {
    return { count: result }
  } else {
    return result
  }
}

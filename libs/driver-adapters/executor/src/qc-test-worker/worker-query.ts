import * as util from 'node:util'
import {
  SqlQueryable,
  IsolationLevel,
  ColumnType,
  ColumnTypeEnum,
  SqlResultSet,
} from '@prisma/driver-adapter-utils'
import { JsonProtocolQuery, QueryParams } from '../types/jsonRpc'
import type { State } from './worker'
import { assertNever, debug } from '../utils'
import {
  noopTracingHelper,
  QueryEvent,
  QueryInterpreter,
  type QueryInterpreterTransactionManager,
  QueryPlanNode,
  type TransactionManager,
  UserFacingError,
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
        ? this.transactionManager.getTransaction({ id: txId }, 'query')
        : this.driverAdapter

      if (!queryable) {
        throw new Error(
          `No transaction with id ${txId} found. Please call 'startTx' first.`,
        )
      }

      try {
        const result = await this.executeQuery(queryable, query, !txId)

        debug('🟢 Query result: ', result)

        return JSON.stringify(getResponseInQeFormat(query, result))
      } catch (error) {
        if (error instanceof UserFacingError) {
          return JSON.stringify({
            errors: [error.toQueryResponseErrorObject()],
          })
        }
        throw error
      }
    }
  }

  private async executeQuery(
    queryable: SqlQueryable,
    query: JsonProtocolQuery,
    allowTransaction: boolean,
  ) {
    let queryPlanString: string
    try {
      queryPlanString = withLocalPanicHandler(() =>
        this.compiler.compile(JSON.stringify(query)),
      )
    } catch (error) {
      if (typeof error.message === 'string' && typeof error.code === 'string') {
        throw new UserFacingError(error.message, error.code, error.meta)
      } else {
        throw error
      }
    }

    const queryPlan = JSON.parse(queryPlanString) as QueryPlanNode

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
        this.logs.push(JSON.stringify(event))
      },
      tracingHelper: noopTracingHelper,
    }

    // workaround needed due to raw SQL tests being written against unserialized results
    const interpreter = isPlainRawQuery(queryPlan)
      ? new QueryInterpreter({
          ...interpreterOpts,
          serializer: serializeRawQueryResult,
        })
      : QueryInterpreter.forSql(interpreterOpts)

    return interpreter.run(queryPlan, queryable)
  }

  private async executeIndependentBatch(queries: readonly JsonProtocolQuery[]) {
    return Promise.all(
      queries.map((query) =>
        this.executeQuery(this.driverAdapter, query, true),
      ),
    )
  }

  private async executeTransactionalBatch(
    queries: readonly JsonProtocolQuery[],
    isolationLevel?: IsolationLevel,
  ) {
    const txInfo = await this.transactionManager.startTransaction({
      maxWait: 200,
      timeout: 500,
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
      [getFullOperationName(query)]: getOperationResultInQeFormat(result),
    },
  }
}

function getFullOperationName(query: JsonProtocolQuery): string {
  switch (query.action) {
    case 'createManyAndReturn':
      return `createMany${query.modelName}AndReturn`
    case 'updateManyAndReturn':
      return `updateMany${query.modelName}AndReturn`
    default:
      if (query.modelName) {
        return query.action + query.modelName
      } else {
        return query.action
      }
  }
}

function getOperationResultInQeFormat(result: unknown) {
  if (typeof result === 'number') {
    return { count: result }
  } else {
    return result
  }
}

function isPlainRawQuery(plan: QueryPlanNode): boolean {
  switch (plan.type) {
    case 'query':
      return plan.args.type === 'rawSql'
    case 'seq':
      return plan.args.length === 1 && isPlainRawQuery(plan.args[0])
    default:
      return false
  }
}

function serializeRawQueryResult(
  resultSet: SqlResultSet,
): Record<string, unknown> {
  return {
    columns: resultSet.columnNames,
    types: resultSet.columnTypes.map((type) => serializeColumnType(type)),
    rows: resultSet.rows,
  }
}

// maps JS column types to their Rust equivalents in order to satisfy assertions in tests
function serializeColumnType(columnType: ColumnType): string {
  switch (columnType) {
    case ColumnTypeEnum.Int32:
      return 'int'
    case ColumnTypeEnum.Int64:
      return 'bigint'
    case ColumnTypeEnum.Float:
      return 'float'
    case ColumnTypeEnum.Double:
      return 'double'
    case ColumnTypeEnum.Text:
      return 'string'
    case ColumnTypeEnum.Enum:
      return 'enum'
    case ColumnTypeEnum.Bytes:
      return 'bytes'
    case ColumnTypeEnum.Boolean:
      return 'bool'
    case ColumnTypeEnum.Character:
      return 'char'
    case ColumnTypeEnum.Numeric:
      return 'decimal'
    case ColumnTypeEnum.Json:
      return 'json'
    case ColumnTypeEnum.Uuid:
      return 'uuid'
    case ColumnTypeEnum.DateTime:
      return 'datetime'
    case ColumnTypeEnum.Date:
      return 'date'
    case ColumnTypeEnum.Time:
      return 'time'
    case ColumnTypeEnum.Int32Array:
      return 'int-array'
    case ColumnTypeEnum.Int64Array:
      return 'bigint-array'
    case ColumnTypeEnum.FloatArray:
      return 'float-array'
    case ColumnTypeEnum.DoubleArray:
      return 'double-array'
    case ColumnTypeEnum.TextArray:
      return 'string-array'
    case ColumnTypeEnum.EnumArray:
      return 'string-array'
    case ColumnTypeEnum.BytesArray:
      return 'bytes-array'
    case ColumnTypeEnum.BooleanArray:
      return 'bool-array'
    case ColumnTypeEnum.CharacterArray:
      return 'char-array'
    case ColumnTypeEnum.NumericArray:
      return 'decimal-array'
    case ColumnTypeEnum.JsonArray:
      return 'json-array'
    case ColumnTypeEnum.UuidArray:
      return 'uuid-array'
    case ColumnTypeEnum.DateTimeArray:
      return 'datetime-array'
    case ColumnTypeEnum.DateArray:
      return 'date-array'
    case ColumnTypeEnum.TimeArray:
      return 'time-array'
    case ColumnTypeEnum.UnknownNumber:
      return 'unknown'
    case ColumnTypeEnum.Set:
      /// The following PlanetScale type IDs are mapped into Set:
      /// - SET (SET) -> e.g. `"foo,bar"` (String-encoded, comma-separated)
      return 'string'
    default:
      assertNever(columnType, `Unexpected column type: ${columnType}`)
  }
}

import type TiDBCloud from '@tidbcloud/serverless'
import { Debug, ok } from '@prisma/driver-adapter-utils'
import type {
  DriverAdapter,
  ResultSet,
  Query,
  Queryable,
  Transaction,
  Result,
  TransactionOptions,
} from '@prisma/driver-adapter-utils'
import {type TiDBCloudColumnType,fieldToColumnType} from './conversion'

const debug = Debug('prisma:driver-adapter:tidbcloud')

class RollbackError extends Error {
  constructor() {
    super('ROLLBACK')
    this.name = 'RollbackError'

    if (Error.captureStackTrace) {
      Error.captureStackTrace(this, RollbackError)
    }
  }
}

class TiDBCloudQueryable<ClientT extends TiDBCloud.Connection | TiDBCloud.Tx> implements Queryable {
  readonly flavour = 'mysql'
  constructor(protected client: ClientT) {}

  /**
   * Execute a query given as SQL, interpolating the given parameters.
   */
  async queryRaw(query: Query): Promise<Result<ResultSet>> {
    const tag = '[js::query_raw]'
    debug(`${tag} %O`, query)

    const result = await this.performIO(query)
    const fields = result.types as TiDBCloud.Types
    const rows = result.rows as TiDBCloud.Row[]
    const lastInsertId = result.lastInsertId?.toString()

    const columnNames = Object.keys(fields) as string[]
    const columnRawTypes =  Object.values(fields) as string[]

    const resultSet: ResultSet = {
      columnNames,
      columnTypes: columnRawTypes.map((field) => fieldToColumnType(field as TiDBCloudColumnType)),
      rows: rows as ResultSet['rows'],
      lastInsertId
    }
    return ok(resultSet)
  }

  /**
   * Execute a query given as SQL, interpolating the given parameters and
   * returning the number of affected rows.
   * Note: Queryable expects a u64, but napi.rs only supports u32.
   */
  async executeRaw(query: Query): Promise<Result<number>> {
    const tag = '[js::execute_raw]'
    debug(`${tag} %O`, query)

    const result = await this.performIO(query)
    const rowsAffected = result.rowsAffected as number
    return ok(rowsAffected)
  }

  /**
   * Run a query against the database, returning the result set.
   * Should the query fail due to a connection error, the connection is
   * marked as unhealthy.
   */
  private async performIO(query: Query) {
    const { sql, args: values } = query

    try {
      const result =  await this.client.execute(sql, values, {arrayMode: true, fullResult: true})
      return result as TiDBCloud.FullResult
    } catch (e) {
      const error = e as Error
      debug('Error in performIO: %O', error)
      throw error
    }
  }
}

class TiDBCloudTransaction extends TiDBCloudQueryable<TiDBCloud.Tx> implements Transaction {
  finished = false

  constructor(
    tx: TiDBCloud.Tx,
    readonly options: TransactionOptions,
  ) {
    super(tx)
  }

  async commit(): Promise<Result<void>> {
    debug(`[js::commit]`)

    this.finished = true
    await this.client.commit()
    return Promise.resolve(ok(undefined))
  }

  async rollback(): Promise<Result<void>> {
    debug(`[js::rollback]`)

    this.finished = true
    await this.client.rollback()
    return Promise.resolve(ok(undefined))
  }

  dispose(): Result<void> {
    if (!this.finished) {
      this.rollback().catch(console.error)
    }
    return ok(undefined)
  }
}

export class PrismaTiDBCloud extends TiDBCloudQueryable<TiDBCloud.Connection> implements DriverAdapter {
  constructor(client: TiDBCloud.Connection) {
    super(client)
  }

  async startTransaction() {
    const options: TransactionOptions = {
      usePhantomQuery: true,
    }

    const tag = '[js::startTransaction]'
    debug(`${tag} options: %O`, options)

    const tx = await this.client.begin()
    return ok(new TiDBCloudTransaction(tx, options))
  }

  async close() {
    return ok(undefined)
  }
}

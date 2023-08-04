import { Client, neon, neonConfig } from '@neondatabase/serverless'
import type { NeonConfig, NeonQueryFunction } from '@neondatabase/serverless'
import ws from 'ws'
import { binder, isConnectionUnhealthy, Debug } from '@jkomyno/prisma-js-connector-utils'
import type { Closeable, Connector, ResultSet, Query, ConnectorConfig } from '@jkomyno/prisma-js-connector-utils'
import { fieldToColumnType } from './conversion'

neonConfig.webSocketConstructor = ws

const debug = Debug('prisma:js-connector:neon')

export type PrismaNeonConfig = ConnectorConfig & Partial<Omit<NeonConfig, 'connectionString'>> & { httpMode?: boolean }

const TRANSACTION_BEGIN = 'BEGIN'
const TRANSACTION_COMMIT = 'COMMIT'
const TRANSACTION_ROLLBACK = 'ROLLBACK'

class PrismaNeon implements Connector, Closeable {
  readonly flavor = 'postgres'

  private client?: Client
  private httpClient?: NeonQueryFunction<false, true>
  private isRunning: boolean = true
  private inTransaction: boolean = false
  private _isHealthy: boolean = true
  private _version: string | undefined = undefined
  private _httpMode: boolean

  constructor(config: PrismaNeonConfig) {
    const { url: connectionString, httpMode, ...rest } = config
    this._httpMode = httpMode ?? false
    if (!this._httpMode) {
      this.client = new Client({ connectionString, ...rest })
      // connect the client in the background, all requests will be queued until connection established
      this.client.connect()
    } else {
      this.httpClient = neon(connectionString, { fullResults: true, ...rest })
    }
  }

  async close(): Promise<void> {
    if (this.isRunning) {
      if (!this._httpMode) {
        await this.client!.end()
      }
      this.isRunning = false
    }
  }

  /**
   * Returns false, if connection is considered to not be in a working state.
   */
  isHealthy(): boolean {
    return this.isRunning && this._isHealthy
  }

  /**
   * Execute a query given as SQL, interpolating the given parameters.
   */
  async queryRaw(query: Query): Promise<ResultSet> {
    const tag = '[js::query_raw]'
    debug(`${tag} %O`, query)

    const { fields, rows: results } = await this.performIO(query)

    const columns = fields.map(field => field.name)
    const resultSet: ResultSet = {
      columnNames: columns,
      columnTypes: fields.map(field => fieldToColumnType(field.dataTypeID)),
      rows: results.map(result => columns.map(column => result[column])),
    }

    return resultSet
  }

  /**
   * Execute a query given as SQL, interpolating the given parameters and
   * returning the number of affected rows.
   * Note: Queryable expects a u64, but napi.rs only supports u32.
   */
  async executeRaw(query: Query): Promise<number> {
    const tag = '[js::execute_raw]'
    debug(`${tag} %O`, query)

    switch (query.sql) {
      case TRANSACTION_BEGIN: {
        if (this._httpMode) {
          throw new Error('Transactions are not supported in HTTP mode')
        }

        // check if a transaction is already in progress
        if (this.inTransaction) {
          throw new Error('A transaction is already in progress')
        }

        this.inTransaction = true
        debug(`${tag} transaction began`)

        return Promise.resolve(-1)
      }
      case TRANSACTION_COMMIT: {
        this.inTransaction = false
        debug(`${tag} transaction ended successfully`)
        return Promise.resolve(-1)
      }
      case TRANSACTION_ROLLBACK: {
        this.inTransaction = false
        debug(`${tag} transaction ended with error`)
        return Promise.reject(query.sql)
      }
      default: {
        const { rowCount: rowsAffected } = await this.performIO(query)
        return rowsAffected
      }
    }
  }

  /**
   * Return the version of the underlying database, queried directly from the
   * source. This corresponds to the `version()` function on PostgreSQL for
   * example. The version string is returned directly without any form of
   * parsing or normalization.
   */
  async version(): Promise<string | undefined> {
    if (this._version) {
      return Promise.resolve(this._version)
    }

    const { rows } = await this.performIO({ sql: 'SELECT VERSION()', args: [] })
    this._version = rows[0]['version'] as string
    return this._version
  }

  /**
   * Run a query against the database, returning the result set.
   * Should the query fail due to a connection error, the connection is
   * marked as unhealthy.
   */
  private async performIO(query: Query) {
    const { sql, args: values } = query

    try {
      if (this._httpMode) {
        return await this.client!.query(sql, values)
      } else {
        return await this.httpClient!(sql, values)
      }
    } catch (e) {
      const error = e as Error & { code: string }

      if (isConnectionUnhealthy(error.code)) {
        this._isHealthy = false
      }

      throw e
    }
  }
}

export const createNeonConnector = (config: PrismaNeonConfig): Connector & Closeable => {
  const db = new PrismaNeon(config)
  return binder(db)
}

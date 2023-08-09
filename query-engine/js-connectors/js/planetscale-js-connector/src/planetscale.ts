import * as planetScale from '@planetscale/database'
import type { Config as PlanetScaleConfig } from '@planetscale/database'
import { EventEmitter } from 'node:events'
import { setImmediate } from 'node:timers/promises'
import { binder, isConnectionUnhealthy, Debug } from '@jkomyno/prisma-js-connector-utils'
import type { Connector, ResultSet, Query, ConnectorConfig } from '@jkomyno/prisma-js-connector-utils'
import { type PlanetScaleColumnType, fieldToColumnType } from './conversion'

const debug = Debug('prisma:js-connector:planetscale')

export type PrismaPlanetScaleConfig = ConnectorConfig & Partial<PlanetScaleConfig>

type TransactionCapableDriver
  = {
    /**
     * Indicates a transaction is in progress in this connector's instance.
     */
    inTransaction: true

    /**
     * The PlanetScale client, scoped in transaction mode.
     */
    client: planetScale.Transaction
  }
  | {
    /**
     * Indicates that no transactions are in progress in this connector's instance.
     */
    inTransaction: false

    /**
     * The standard PlanetScale client.
     */
    client: planetScale.Connection
  }

const TRANSACTION_BEGIN = 'BEGIN'
const TRANSACTION_COMMIT = 'COMMIT'
const TRANSACTION_ROLLBACK = 'ROLLBACK'

class PrismaPlanetScale implements Connector {
  readonly flavour = 'mysql'
  
  private driver: TransactionCapableDriver
  private isRunning: boolean = true
  private _isHealthy: boolean = true
  private _version: string | undefined = undefined
  private txEmitter = new EventEmitter()

  constructor(config: PrismaPlanetScaleConfig) {
    const client = planetScale.connect(config)

    // initialize the driver as a non-transactional client
    this.driver = {
      client,
      inTransaction: false,
    }
  }

  async close(): Promise<void> {
    if (this.isRunning) {
      this.isRunning = false
    }
  }

  /**
   * Returns true, if connection is considered to be in a working state.
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

    const { fields, insertId: lastInsertId, rows: results } = await this.performIO(query)

    const columns = fields.map(field => field.name)
    const resultSet: ResultSet = {
      columnNames: columns,
      columnTypes: fields.map(field => fieldToColumnType(field.type as PlanetScaleColumnType)),
      rows: results.map(result => columns.map(column => result[column])),
      lastInsertId,
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

    const connection = this.driver.client
    const { sql } = query

    switch (sql) {
      case TRANSACTION_BEGIN: {
        // check if a transaction is already in progress
        if (this.driver.inTransaction) {
          throw new Error('A transaction is already in progress')
        }

        (this.driver.client as planetScale.Connection).transaction(async (tx) => {
          // tx holds the scope for executing queries in transaction mode
          this.driver.client = tx
  
          // signal the transaction began
          this.driver.inTransaction = true
          debug(`${tag} transaction began`)

          await new Promise((resolve, reject) => {
            this.txEmitter.once(TRANSACTION_COMMIT, () => {
              this.driver.inTransaction = false
              debug(`${tag} transaction ended successfully`)
              this.driver.client = connection
              resolve(undefined)
            })
  
            this.txEmitter.once(TRANSACTION_ROLLBACK, () => {
              this.driver.inTransaction = false
              debug(`${tag} transaction ended with error`)
              this.driver.client = connection
              reject('ROLLBACK')
            })
          })
        })
  
        // ensure that this.driver.client is set to `planetScale.Transaction`
        await setImmediate(0, {
          // we do not require the event loop to remain active
          ref: false,
        })
  
        return Promise.resolve(-1)
      }
      case TRANSACTION_COMMIT: {
        this.txEmitter.emit(sql)
        return Promise.resolve(-1)
      }
      case TRANSACTION_ROLLBACK: {
        this.txEmitter.emit(sql)
        return Promise.resolve(-2)
      }
      default: {
        const { rowsAffected } = await this.performIO(query)
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

    const { rows } = await this.performIO({ sql: 'SELECT @@version', args: [] })
    this._version = rows[0]['@@version'] as string
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
      return await this.driver.client.execute(sql, values)
    } catch (e) {
      const error = e as Error & { code: string }
      
      if (isConnectionUnhealthy(error.code)) {
        this._isHealthy = false
      }

      throw e
    }
  }
}

export const createPlanetScaleConnector = (config: PrismaPlanetScaleConfig): Connector => {
  const db = new PrismaPlanetScale(config)
  return binder(db)
}

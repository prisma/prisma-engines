import { Client, neon, neonConfig } from '@neondatabase/serverless'
import type { NeonConfig, NeonQueryFunction } from '@neondatabase/serverless'
import ws from 'ws'
import { bindConnector, Debug } from '@jkomyno/prisma-js-connector-utils'
import type { Connector, ResultSet, Query, ConnectorConfig } from '@jkomyno/prisma-js-connector-utils'
import { fieldToColumnType } from './conversion'

neonConfig.webSocketConstructor = ws

const debug = Debug('prisma:js-connector:neon')

export type PrismaNeonConfig = ConnectorConfig & Partial<Omit<NeonConfig, 'connectionString'>> & { httpMode?: boolean }

const TRANSACTION_BEGIN = 'BEGIN'
const TRANSACTION_COMMIT = 'COMMIT'
const TRANSACTION_ROLLBACK = 'ROLLBACK'

type ARRAY_MODE_DISABLED = false
type FULL_RESULTS_ENABLED = true

type ModeSpecificDriver
  = {
    /**
     * Indicates that we're using the HTTP mode.
     */
    mode: 'http'

    /**
     * The Neon HTTP client, without transaction support.
     */
    client: NeonQueryFunction<ARRAY_MODE_DISABLED, FULL_RESULTS_ENABLED>
  }
  | {
    /**
     * Indicates that we're using the WebSocket mode.
     */
    mode: 'ws'

    /**
     * The standard Neon client, with transaction support.
     */
    client: Client
  }

class PrismaNeon implements Connector {
  readonly flavour = 'postgres'

  private driver: ModeSpecificDriver
  private isRunning: boolean = true
  private inTransaction: boolean = false

  constructor(config: PrismaNeonConfig) {
    const { url: connectionString, httpMode, ...rest } = config
    if (!httpMode) {
      this.driver = {
        mode: 'ws',
        client: new Client({ connectionString, ...rest })
      }
      // connect the client in the background, all requests will be queued until connection established
      this.driver.client.connect()
    } else {
      this.driver = {
        mode: 'http',
        client: neon(connectionString, { fullResults: true, ...rest })
      }
    }
  }

  async close(): Promise<void> {
    if (this.isRunning) {
      if (this.driver.mode === 'ws') {
        await this.driver.client.end()
      }
      this.isRunning = false
    }
  }

  async startTransaction(isolationLevel?: string) {
    return {} as any
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
        if (this.driver.mode === 'http') {
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
   * Run a query against the database, returning the result set.
   * Should the query fail due to a connection error, the connection is
   * marked as unhealthy.
   */
  private async performIO(query: Query) {
    const { sql, args: values } = query

    if (this.driver.mode === 'ws') {
      return await this.driver.client.query(sql, values)
    } else {
      return await this.driver.client(sql, values)
    }
  }
}

export const createNeonConnector = (config: PrismaNeonConfig): Connector => {
  const db = new PrismaNeon(config)
  return bindConnector(db)
}

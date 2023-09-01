import * as pg from 'pg'
import { bindConnector, Debug } from '@jkomyno/prisma-js-connector-utils'
import type { ErrorCapturingConnector, Connector, ConnectorConfig, Query, Queryable, Result, ResultSet, Transaction, TransactionOptions } from '@jkomyno/prisma-js-connector-utils'
import { fieldToColumnType } from './conversion'

const debug = Debug('prisma:js-connector:pg')

export type PrismaPgConfig = ConnectorConfig

type StdClient = pg.Pool
type TransactionClient = pg.PoolClient

class PgQueryable<ClientT extends StdClient | TransactionClient>
  implements Queryable {
  readonly flavour = 'postgres'

  constructor(protected readonly client: ClientT) {
  }

  /**
   * Execute a query given as SQL, interpolating the given parameters.
   */
  async queryRaw(query: Query): Promise<Result<ResultSet>> {
    const tag = '[js::query_raw]'
    debug(`${tag} %O`, query)

    const { fields, rows: results } = await this.performIO(query)

    const columns = fields.map((field) => field.name)
    const resultSet: ResultSet = {
      columnNames: columns,
      columnTypes: fields.map((field) => fieldToColumnType(field.dataTypeID)),
      rows: results.map((result) => columns.map((column) => result[column])),
    }

    return { ok: true, value: resultSet }
  }

  /**
   * Execute a query given as SQL, interpolating the given parameters and
   * returning the number of affected rows.
   * Note: Queryable expects a u64, but napi.rs only supports u32.
   */
  async executeRaw(query: Query): Promise<Result<number>> {
    const tag = '[js::execute_raw]'
    debug(`${tag} %O`, query)

    const { rowCount } = await this.performIO(query)
    return { ok: true, value: rowCount }
  }

  /**
   * Run a query against the database, returning the result set.
   * Should the query fail due to a connection error, the connection is
   * marked as unhealthy.
   */
  private async performIO(query: Query) {
    const { sql, args: values } = query

    try {
      const result = await this.client.query(sql, values)
      return result
    } catch (e) {
      const error = e as Error
      debug('Error in performIO: %O', error)
      throw error
    }
  }
}

class PgTransaction extends PgQueryable<TransactionClient>
  implements Transaction {
  constructor(client: pg.PoolClient, readonly options: TransactionOptions) {
    super(client)
  }

  async commit(): Promise<Result<void>> {
    const tag = '[js::commit]'
    debug(`${tag} committing transaction`)

    try {
      await this.client.query('COMMIT')
      return { ok: true, value: undefined }
    } finally {
      this.client.release()
    }
  }

  async rollback(): Promise<Result<void>> {
    const tag = '[js::rollback]'
    debug(`${tag} rolling back the transaction`)

    try {
      await this.client.query('ROLLBACK')
      return { ok: true, value: undefined }
    } finally {
      this.client.release()
    }
  }
}

class PrismaPg extends PgQueryable<StdClient> implements Connector {
  constructor(config: PrismaPgConfig) {
    const { url: connectionString } = config
    
    const client = new pg.Pool({
      connectionString,
    })

    super(client)
  }

  async startTransaction(isolationLevel?: string): Promise<Result<Transaction>> {
    const options: TransactionOptions = {
      isolationLevel,
      isolationFirst: false,
      usePhantomQuery: false,
    }

    const tag = '[js::startTransaction]'
    debug(`${tag} options: %O`, options)

    const connection = await this.client.connect()
    return { ok: true, value: new PgTransaction(connection, options) }
  }

  async close() {
    return { ok: true as const, value: undefined }
  }
}

export const createPgConnector = (config: PrismaPgConfig): ErrorCapturingConnector => {
  const db = new PrismaPg(config)
  return bindConnector(db)
}

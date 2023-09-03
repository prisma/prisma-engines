import { FullQueryResults, PoolClient, neon, neonConfig } from '@neondatabase/serverless'
import { NeonConfig, NeonQueryFunction, Pool, QueryResult } from '@neondatabase/serverless'
import ws from 'ws'
import { bindConnector, Debug } from '@jkomyno/prisma-js-connector-utils'
import type { Connector, ResultSet, Query, ConnectorConfig, Queryable, Transaction, Result, ErrorCapturingConnector } from '@jkomyno/prisma-js-connector-utils'
import { fieldToColumnType } from './conversion'

neonConfig.webSocketConstructor = ws

const debug = Debug('prisma:js-connector:neon')

export type PrismaNeonConfig = ConnectorConfig & Partial<Omit<NeonConfig, 'connectionString'>> & { httpMode?: boolean }

type ARRAY_MODE_DISABLED = false
type FULL_RESULTS_ENABLED = true

type PerformIOResult = QueryResult<any> | FullQueryResults<ARRAY_MODE_DISABLED> 

/**
 * Base class for http client, ws client and ws transaction
 */
abstract class NeonQueryable implements Queryable {
  flavour = 'postgres' as const

  async queryRaw(query: Query): Promise<Result<ResultSet>> {
    const tag = '[js::query_raw]'
    debug(`${tag} %O`, query)

    const { fields, rows: results } = await this.performIO(query)

    const columns = fields.map(field => field.name)
    const resultSet: ResultSet = {
      columnNames: columns,
      columnTypes: fields.map(field => fieldToColumnType(field.dataTypeID)),
      rows: results.map(result => columns.map(column => result[column])),
    }

    return { ok: true, value: resultSet }
  }

  async executeRaw(query: Query): Promise<Result<number>> {
    const tag = '[js::execute_raw]'
    debug(`${tag} %O`, query)

    const { rowCount: rowsAffected } = await this.performIO(query)
    return { ok: true, value: rowsAffected }
  }

  abstract performIO(query: Query): Promise<PerformIOResult>
}

/**
 * Base class for WS-based queryables: top-level client and transaction
 */
class NeonWsQueryable<ClientT extends Pool|PoolClient> extends NeonQueryable {
  constructor(protected client: ClientT) {
    super()
  }

  override async performIO(query: Query): Promise<PerformIOResult> {
    const { sql, args: values } = query

    try {
      return await this.client.query(sql, values)
    } catch (e) {
      const error = e as Error
      debug('Error in performIO: %O', error)
      throw error
    }
  }
}

class NeonTransaction extends NeonWsQueryable<PoolClient> implements Transaction {
  async commit(): Promise<Result<void>> {
    try {
      await this.client.query('COMMIT');
      return { ok: true, value: undefined }
    } finally {
      this.client.release()
    }
  }

  async rollback(): Promise<Result<void>> {
    try {
      await this.client.query('ROLLBACK');
      return { ok: true, value: undefined }
    } finally {
      this.client.release()
    }
  }

}

class NeonWsConnector extends NeonWsQueryable<Pool> implements Connector {
  private isRunning = true
  constructor(config: PrismaNeonConfig) {
    const { url: connectionString, httpMode, ...rest } = config
    super(new Pool({ connectionString, ...rest }))
  }

  async startTransaction(isolationLevel?: string | undefined): Promise<Result<Transaction>> {
    const connection = await this.client.connect()
    await connection.query('BEGIN')
    if (isolationLevel) {
      await connection.query(`SET TRANSACTION ISOLATION LEVEL ${isolationLevel}`)
    }

    return { ok: true, value: new NeonTransaction(connection) }
  }

  async close() {
    if (this.isRunning) {
      await this.client.end()
      this.isRunning = false
    }
    return { ok: true as const, value: undefined }
  }
}

class NeonHttpConnector extends NeonQueryable implements Connector {
  private client: NeonQueryFunction<ARRAY_MODE_DISABLED, FULL_RESULTS_ENABLED>

  constructor(config: PrismaNeonConfig) {
    super()
    const { url: connectionString, httpMode, ...rest } = config
    this.client = neon(connectionString, { fullResults: true, ...rest})
  }

  override async performIO(query: Query): Promise<PerformIOResult> {
    const { sql, args: values } = query
      return await this.client(sql, values)
  }

  startTransaction(): Promise<Result<Transaction>> {
    return Promise.reject(new Error('Transactions are not supported in HTTP mode'))
  }

  async close() {
    return { ok: true as const, value: undefined }
  }

}

export const createNeonConnector = (config: PrismaNeonConfig): ErrorCapturingConnector => {
  const db = config.httpMode ? new NeonHttpConnector(config) : new NeonWsConnector(config)
  return bindConnector(db)
}

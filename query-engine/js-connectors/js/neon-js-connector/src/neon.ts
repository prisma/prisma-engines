import { FullQueryResults, PoolClient, neon, neonConfig } from '@neondatabase/serverless'
import { NeonConfig, NeonQueryFunction, Pool, QueryResult } from '@neondatabase/serverless'
import ws from 'ws'
import { bindConnector, Debug } from '@jkomyno/prisma-js-connector-utils'
import type { Connector, ResultSet, Query, ConnectorConfig, Queryable, Transaction, Result, ErrorCapturingConnector, TransactionOptions } from '@jkomyno/prisma-js-connector-utils'
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
    
    // Note: `rowsAffected` can sometimes be null (e.g., when executing `"BEGIN"`)
    return { ok: true, value: rowsAffected ?? 0 }
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
  constructor(client: PoolClient, readonly options: TransactionOptions) {
    super(client)
  }

  async commit(): Promise<Result<void>> {
    debug(`[js::commit]`)

    this.client.release()
    return Promise.resolve({ ok: true, value: undefined })
  }

  async rollback(): Promise<Result<void>> {
    debug(`[js::rollback]`)

    this.client.release()
    return Promise.resolve({ ok: true, value: undefined })
  }
}

class NeonWsConnector extends NeonWsQueryable<Pool> implements Connector {
  private isRunning = true
  constructor(config: PrismaNeonConfig) {
    const { url: connectionString, httpMode, ...rest } = config
    super(new Pool({ connectionString, ...rest }))
  }

  async startTransaction(isolationLevel?: string | undefined): Promise<Result<Transaction>> {
    const options: TransactionOptions = {
      isolationLevel,
      isolationFirst: false,
      usePhantomQuery: false,
    }
    
    const tag = '[js::startTransaction]'
    debug(`${tag} options: %O`, options)
    
    const connection = await this.client.connect()
    return { ok: true, value: new NeonTransaction(connection, options) }
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

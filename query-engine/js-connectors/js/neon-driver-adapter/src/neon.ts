import type neon from '@neondatabase/serverless'
import { bindTransaction, Debug } from '@jkomyno/prisma-js-connector-utils'
import type {
  Connector,
  Query,
  Queryable,
  ResultSet,
  Transaction,
} from '@jkomyno/prisma-js-connector-utils'
import { fieldToColumnType } from './conversion'

const debug = Debug('prisma:js-connector:neon')

type ARRAY_MODE_DISABLED = false
type FULL_RESULTS_ENABLED = true

type PerformIOResult = neon.QueryResult<any> | neon.FullQueryResults<ARRAY_MODE_DISABLED>

/**
 * Base class for http client, ws client and ws transaction
 */
abstract class NeonQueryable implements Queryable {
  flavour = 'postgres' as const

  async queryRaw(query: Query): Promise<ResultSet> {
    const tag = '[js::query_raw]'
    debug(`${tag} %O`, query)

    const { fields, rows: results } = await this.performIO(query)

    const columns = fields.map((field) => field.name)
    const resultSet: ResultSet = {
      columnNames: columns,
      columnTypes: fields.map((field) => fieldToColumnType(field.dataTypeID)),
      rows: results.map((result) => columns.map((column) => result[column])),
    }

    return resultSet
  }

  async executeRaw(query: Query): Promise<number> {
    const tag = '[js::execute_raw]'
    debug(`${tag} %O`, query)

    const { rowCount: rowsAffected } = await this.performIO(query)
    return rowsAffected
  }

  abstract performIO(query: Query): Promise<PerformIOResult>
}

/**
 * Base class for WS-based queryables: top-level client and transaction
 */
class NeonWsQueryable<ClientT extends neon.Pool | neon.PoolClient> extends NeonQueryable {
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

class NeonTransaction extends NeonWsQueryable<neon.PoolClient>
  implements Transaction {
  async commit(): Promise<void> {
    try {
      await this.client.query('COMMIT')
    } finally {
      this.client.release()
    }
  }

  async rollback(): Promise<void> {
    try {
      await this.client.query('ROLLBACK')
    } finally {
      this.client.release()
    }
  }
}

export class NeonWSAdapter extends NeonWsQueryable<neon.Pool> implements Connector {
  private isRunning = true

  constructor(pool: neon.Pool) {
    super(pool)
  }

  async startTransaction(
    isolationLevel?: string | undefined,
  ): Promise<Transaction> {
    const connection = await this.client.connect()
    await connection.query('BEGIN')
    if (isolationLevel) {
      await connection.query(
        `SET TRANSACTION ISOLATION LEVEL ${isolationLevel}`,
      )
    }

    return bindTransaction(new NeonTransaction(connection))
  }

  async close() {
    this.client.on('error', (e) => console.log(e))
    if (this.isRunning) {
      await this.client.end()
      this.isRunning = false
    }
  }
}

export class NeonHTTPAdapter extends NeonQueryable implements Connector {
  constructor(
    private client: neon.NeonQueryFunction<
      ARRAY_MODE_DISABLED,
      FULL_RESULTS_ENABLED
    >,
  ) {
    super()
  }

  override async performIO(query: Query): Promise<PerformIOResult> {
    const { sql, args: values } = query
    return await this.client(sql, values)
  }

  startTransaction(): Promise<Transaction> {
    return Promise.reject(
      new Error('Transactions are not supported in HTTP mode'),
    )
  }

  async close() {}
}

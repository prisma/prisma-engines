import { ColumnTypeEnum, Debug } from '@prisma/driver-adapter-utils'
import type {
  DriverAdapter,
  Query,
  Queryable,
  Result,
  ResultSet,
  Transaction,
  TransactionOptions,
} from '@prisma/driver-adapter-utils'
import type { InStatement, Client as LibsqlClientRaw, Transaction as LibsqlTransactionRaw } from '@libsql/client'
import { fieldToColumnType } from './conversion'

const debug = Debug('prisma:driver-adapter:libsql')

type StdClient = LibsqlClientRaw
type TransactionClient = LibsqlTransactionRaw

class LibsqlQueryable<ClientT extends StdClient | TransactionClient> implements Queryable {
  readonly flavour = 'sqlite'

  constructor(protected readonly client: ClientT) {}

  /**
   * Execute a query given as SQL, interpolating the given parameters.
   */
  async queryRaw(query: Query): Promise<Result<ResultSet>> {
    const tag = '[js::query_raw]'
    debug(`${tag} %O`, query)

    const { columns, rows } = await this.performIO(query)

    // HACK: since decltype isn't exposed, we have no way to even infer types if there are no rows
    const columnTypes =
      rows.length > 0 ? Array.from(rows[0]).map(fieldToColumnType) : columns.map(() => ColumnTypeEnum.Int32)

    const resultSet: ResultSet = {
      columnNames: columns,
      columnTypes,
      rows: rows.map((row) => Array.from(row)),
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

    const { rowsAffected } = await this.performIO(query)
    return { ok: true, value: rowsAffected }
  }

  /**
   * Run a query against the database, returning the result set.
   * Should the query fail due to a connection error, the connection is
   * marked as unhealthy.
   */
  private async performIO(query: Query) {
    try {
      // TODO: type assertion: are driver adapter query args always compatible with libsql's InValue?
      // ```
      // export type Value = null | string | number | bigint | ArrayBuffer;
      // export type InValue = Value | boolean | Uint8Array | Date;
      // ```
      const result = await this.client.execute(query as InStatement)
      return result
    } catch (e) {
      const error = e as Error
      debug('Error in performIO: %O', error)
      throw error
    }
  }
}

class LibsqlTransaction extends LibsqlQueryable<TransactionClient> implements Transaction {
  constructor(client: TransactionClient, readonly options: TransactionOptions) {
    super(client)
  }

  async commit(): Promise<Result<void>> {
    debug(`[js::commit]`)

    await this.client.commit()
    return Promise.resolve({ ok: true, value: undefined })
  }

  async rollback(): Promise<Result<void>> {
    debug(`[js::rollback]`)

    this.client.rollback()
    return Promise.resolve({ ok: true, value: undefined })
  }
}

export class PrismaLibsql extends LibsqlQueryable<StdClient> implements DriverAdapter {
  constructor(client: StdClient) {
    super(client)
  }

  async startTransaction(): Promise<Result<Transaction>> {
    const options: TransactionOptions = {
      usePhantomQuery: false,
    }

    const tag = '[js::startTransaction]'
    debug(`${tag} options: %O`, options)

    const tx = await this.client.transaction('write')
    return { ok: true, value: new LibsqlTransaction(tx, options) }
  }

  async close(): Promise<Result<void>> {
    return { ok: true, value: undefined }
  }
}

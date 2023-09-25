import { Debug, ok, err } from '@prisma/driver-adapter-utils'
import type {
  DriverAdapter,
  Query,
  Queryable,
  Result,
  ResultSet,
  Transaction,
  TransactionOptions,
} from '@prisma/driver-adapter-utils'
import type { InStatement, Client as LibSqlClientRaw, Transaction as LibSqlTransactionRaw, ResultSet as LibsqlResultSet } from '@libsql/client'
import { getColumnTypes, mapRow } from './conversion'

const debug = Debug('prisma:driver-adapter:libsql')

type StdClient = LibSqlClientRaw
type TransactionClient = LibSqlTransactionRaw


class LibSqlQueryable<ClientT extends StdClient | TransactionClient> implements Queryable {
  readonly flavour = 'sqlite'

  constructor(protected readonly client: ClientT) {}

  /**
   * Execute a query given as SQL, interpolating the given parameters.
   */
  async queryRaw(query: Query): Promise<Result<ResultSet>> {
    const tag = '[js::query_raw]'
    debug(`${tag} %O`, query)

    return (await this.performIO(query)).map( ({ columns, rows, columnTypes: declaredColumnTypes }) => {
      const columnTypes = getColumnTypes(declaredColumnTypes, rows)

      const resultSet: ResultSet = {
        columnNames: columns,
        columnTypes,
        rows: rows.map(mapRow),
      }

      return resultSet
    })
  }

  /**
   * Execute a query given as SQL, interpolating the given parameters and
   * returning the number of affected rows.
   * Note: Queryable expects a u64, but napi.rs only supports u32.
   */
  async executeRaw(query: Query): Promise<Result<number>> {
    const tag = '[js::execute_raw]'
    debug(`${tag} %O`, query)

    return (await this.performIO(query)).map((r) => r.rowsAffected ?? 0)
  }

  /**
   * Run a query against the database, returning the result set.
   * Should the query fail due to a connection error, the connection is
   * marked as unhealthy.
   */
  private async performIO(query: Query): Promise<Result<LibsqlResultSet>> {
    try {
      return ok(await this.client.execute(query as InStatement))
    } catch (e) {
      console.error('💥 Error in performIO: %O', e)
      if (e && e.code) {
        return err({
          kind: 'SqliteError',
          code: e.code,
          message: e.message,
        })
      } else {
        console.error('❌ Error in performIO: %O. Missing code', e)
      }
      throw e
    }
  }
}

class LibSqlTransaction extends LibSqlQueryable<TransactionClient> implements Transaction {
  constructor(
    client: TransactionClient,
    readonly options: TransactionOptions,
  ) {
    super(client)
  }

  async commit(): Promise<Result<void>> {
    debug(`[js::commit]`)

    await this.client.commit()
    return ok(undefined)
  }

  async rollback(): Promise<Result<void>> {
    debug(`[js::rollback]`)

    try {
      await this.client.rollback()
    } catch (error) {
      debug('error in rollback:', error)
    }

    return ok(undefined)
  }
}

export class PrismaLibSQL extends LibSqlQueryable<StdClient> implements DriverAdapter {
  constructor(client: StdClient) {
    super(client)
  }

  async startTransaction(): Promise<Result<Transaction>> {
    const options: TransactionOptions = {
      usePhantomQuery: true,
    }

    const tag = '[js::startTransaction]'
    debug(`${tag} options: %O`, options)

    const tx = await this.client.transaction('deferred')
    return ok(new LibSqlTransaction(tx, options))
  }

  async close(): Promise<Result<void>> {
    this.client.close()
    return ok(undefined)
  }
}

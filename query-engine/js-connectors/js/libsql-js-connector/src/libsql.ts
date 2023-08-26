import { createClient, Client, Transaction as LibsqlClientTransaction } from "@libsql/client";
import { bindConnector, bindTransaction, Debug } from '@jkomyno/prisma-js-connector-utils'
import type { Connector, ConnectorConfig, Query, Queryable, ResultSet, Transaction } from '@jkomyno/prisma-js-connector-utils'
import { fieldToColumnType } from './conversion'

const debug = Debug('prisma:js-connector:libsql')

export type PrismaLibsqlConfig = ConnectorConfig

type StdClient = Client
type TransactionClient = LibsqlClientTransaction

class LibsqlQueryable<ClientT extends StdClient | TransactionClient>
  implements Queryable {
  readonly flavour = 'postgres'

  constructor(protected readonly client: ClientT) {
  }

  /**
   * Execute a query given as SQL, interpolating the given parameters.
   */
  async queryRaw(query: Query): Promise<ResultSet> {
    const tag = '[js::query_raw]'
    debug(`${tag} %O`, query)

    const { columns: fields, rows: results } = await this.performIO(query)

    const columns = fields //.map((field) => field.name) TODO
    const resultSet: ResultSet = {
      columnNames: columns,
      columnTypes: fields.map((field) => fieldToColumnType(1)), //field.dataTypeID)), TODO
      rows: results.map((result) => columns.map((column) => result[column])),
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

    const { rowsAffected } = await this.performIO(query)
    return rowsAffected
  }

  /**
   * Run a query against the database, returning the result set.
   * Should the query fail due to a connection error, the connection is
   * marked as unhealthy.
   */
  private async performIO(query: Query) {
    const { sql, args: values } = query

    try {
      const result = await this.client.execute(sql) // { sql: sql, args: values }) TODO
      return result
    } catch (e) {
      const error = e as Error
      debug('Error in performIO: %O', error)
      throw error
    }
  }
}

class LibsqlTransaction extends LibsqlQueryable<TransactionClient>
  implements Transaction {
  constructor(client: TransactionClient) {
    super(client)
  }

  async commit(): Promise<void> {
    const tag = '[js::commit]'
    debug(`${tag} committing transaction`)

    try {
      await this.client.execute('COMMIT')
    } finally {
      //this.client.release()
    }
  }

  async rollback(): Promise<void> {
    const tag = '[js::rollback]'
    debug(`${tag} rolling back the transaction`)

    try {
      await this.client.execute('ROLLBACK')
    } finally {
      //this.client.release()
    }
  }
}

class PrismaLibsql extends LibsqlQueryable<StdClient> implements Connector {
  constructor(config: PrismaLibsqlConfig) {
    const { url: connectionString } = config
    
    const client = createClient({
      url: connectionString,
      // authToken: authToken // TODO
    })

    super(client)
  }

  async startTransaction(isolationLevel?: string): Promise<Transaction> {
    const transaction = await this.client.transaction("write");
    
    //const connection = await this.client.connect()
    // await connection.query('BEGIN')

    // if (isolationLevel) {
    //   await connection.query(
    //     `SET TRANSACTION ISOLATION LEVEL ${isolationLevel}`,
    //   )
    // }

    return bindTransaction(new LibsqlTransaction(transaction))
  }

  async close() {}
}

export const createLibsqlConnector = (config: PrismaLibsqlConfig): Connector => {
  const db = new PrismaLibsql(config)
  return bindConnector(db)
}

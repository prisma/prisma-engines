import { createClient, Client, Transaction as LibsqlClientTransaction, InArgs } from "@libsql/client";
import { bindConnector, Debug } from '@jkomyno/prisma-js-connector-utils'
import type { ErrorCapturingConnector, Connector, ConnectorConfig, Query, Queryable, Result, ResultSet, Transaction } from '@jkomyno/prisma-js-connector-utils'
import { resultToColumnType } from './conversion'

const debug = Debug('prisma:js-connector:libsql')

export type PrismaLibsqlConfig = 
{
  url: string;
  token: string;
};
//ConnectorConfig

type StdClient = Client
type TransactionClient = LibsqlClientTransaction

class LibsqlQueryable<ClientT extends StdClient | TransactionClient>
  implements Queryable {
  readonly flavour = 'sqlite'

  constructor(protected readonly client: ClientT) {
  }

  /**
   * Execute a query given as SQL, interpolating the given parameters.
   */
  async queryRaw(query: Query): Promise<Result<ResultSet>> {
    // console.log("### queryRaw")
    const tag = '[js::query_raw]'
    debug(`${tag} %O`, query)

    const { columns: fields, rows: results } = await this.performIO(query)
    // console.log("returned", fields, results)

    // output JS types
    // for (const propName in results[0]) {
    //   if (results[0].hasOwnProperty(propName)) {
    //     console.log(`${propName}: ${typeof results[0][propName]}`);
    //   }
    // }

    let firstResult = {}
    firstResult = results[0]

    let resultSet: ResultSet
    // Handle no results case explicitly TODO Really needed?
    if(firstResult) {
      resultSet = {
        columnNames: fields,
        columnTypes: Object.keys(firstResult).map((key) => resultToColumnType(key, firstResult[key])),
        rows: results.map((result) => fields.map((column) => result[column]))
      };
    } else {
      resultSet = { columnNames: [], columnTypes: [], rows: [] }
    }

    // console.log("resultSet", resultSet)

    return { ok: true, value: resultSet }
  }

  /**
   * Execute a query given as SQL, interpolating the given parameters and
   * returning the number of affected rows.
   * Note: Queryable expects a u64, but napi.rs only supports u32.
   */
  async executeRaw(query: Query): Promise<Result<number>> {
    // console.log("### executeRaw")
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
    const { sql, args: values } = query

    // console.log("### performIO", query)

    try {
      const result = await this.client.execute({ sql: sql, args: values as InArgs })
      // console.log("result", result)
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

  async commit(): Promise<Result<void>> {
    const tag = '[js::commit]'
    debug(`${tag} committing transaction`)

    try {
      await this.client.execute('COMMIT')
      return { ok: true, value: undefined }
    } finally {
      //this.client.release()
    }
  }

  async rollback(): Promise<Result<void>> {
    const tag = '[js::rollback]'
    debug(`${tag} rolling back the transaction`)

    try {
      await this.client.execute('ROLLBACK')
      return { ok: true, value: undefined }
    } finally {
      //this.client.release()
    }
  }
}

class PrismaLibsql extends LibsqlQueryable<StdClient> implements Connector {
  constructor(config: PrismaLibsqlConfig) {
    const { url: connectionString, token: authToken } = config
    
    // console.log("### PrismaLibsql", connectionString, authToken)

    const client = createClient({
      url: connectionString,
      authToken: authToken
    })
    // console.log("Libsql client", client)

    super(client)
  }

  async startTransaction(isolationLevel?: string): Promise<Result<Transaction>> {
    const transaction = await this.client.transaction("write");
    // console.log("startTransaction")
  
    // const connection = await this.client.connect()
    // await connection.query('BEGIN')

    // if (isolationLevel) {
    //   await connection.query(
    //     `SET TRANSACTION ISOLATION LEVEL ${isolationLevel}`,
    //   )
    // }

    return { ok: true, value: new LibsqlTransaction(transaction) }
  }

  async close() {
    return { ok: true as const, value: undefined }
  }
}

export const createLibsqlConnector = (config: PrismaLibsqlConfig): ErrorCapturingConnector => {
  const db = new PrismaLibsql(config)
  return bindConnector(db)
}

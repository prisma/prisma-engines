import * as pg from 'pg'
import { bindConnector, Debug } from '@jkomyno/prisma-js-connector-utils'
import type { ErrorCapturingConnector, Connector, ConnectorConfig, Query, Queryable, Result, ResultSet, Transaction } from '@jkomyno/prisma-js-connector-utils'
import { fieldToColumnType } from './conversion'
import { XataClient } from '../../smoke-test-js/src/xata_gen'

const debug = Debug('prisma:js-connector:xata')

export type PrismaXataConfig = 
{
  // url: string;
  xata: Function;
};
// ConnectorConfig

type StdClient = XataClient
type TransactionClient = XataClient

class XataQueryable<ClientT extends StdClient | TransactionClient>
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

    const results = await this.performIO(query)
    console.log("queryRaw results", results)

    // const columns = fields.map((field) => field.name)
    const resultSet: ResultSet = {
      columnNames: [], // columns,
      columnTypes: [], // fields.map((field) => fieldToColumnType(field.dataTypeID)),
      rows: [], // results.map((result) => columns.map((column) => result[column])),
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

    const results = await this.performIO(query)
    return { ok: true, value: results.records.length }
  }

  /**
   * Run a query against the database, returning the result set.
   * Should the query fail due to a connection error, the connection is
   * marked as unhealthy.
   */
  private async performIO(query: Query) {
    let { sql, args: values } = query
    console.log("### performIO", sql, values)

    try {
      // Remove `"public".` from generate SQL query
      sql = sql.replace(/"public"./g, '')

      const result = await this.client.sql(sql, ...values)
      console.log("performIO result", result)
      return result
    } catch (e) {
      const error = e as Error
      debug('Error in performIO: %O', error)
      throw error
    }
  }
}

class XataTransaction extends XataQueryable<TransactionClient>
  implements Transaction {
  constructor(client: XataClient) {
    super(client)
  }

  async commit(): Promise<Result<void>> {
    // const tag = '[js::commit]'
    // debug(`${tag} committing transaction`)

    // try {
    //   await this.client.query('COMMIT')
    //   return { ok: true, value: undefined }
    // } finally {
    //   //this.client.release()
    // }

    throw new Error("Xata does not support transactions yet")
  }

  async rollback(): Promise<Result<void>> {
    // const tag = '[js::rollback]'
    // debug(`${tag} rolling back the transaction`)

    // try {
    //   await this.client.query('ROLLBACK')
    //   return { ok: true, value: undefined }
    // } finally {
    //   // this.client.release()
    // }

    throw new Error("Xata does not support transactions yet")
  }
}

class PrismaXata extends XataQueryable<StdClient> implements Connector {
  constructor(config: PrismaXataConfig) {
    const { /*url: connectionString, */ xata: xata } = config
    
    // 1
    const client = xata()
    console.log("client", client)

    super(client)
  }

  async startTransaction(isolationLevel?: string): Promise<Result<Transaction>> {
    // const connection = await this.client.connect()
    // await connection.query('BEGIN')

    // if (isolationLevel) {
    //   await connection.query(
    //     `SET TRANSACTION ISOLATION LEVEL ${isolationLevel}`,
    //   )
    // }

    // return { ok: true, value: new XataTransaction(connection) }
    throw new Error("Xata does not support transactions yet")
  }

  async close() {
    return { ok: true as const, value: undefined }
  }
}

export const createXataConnector = (config: PrismaXataConfig): ErrorCapturingConnector => {
  const db = new PrismaXata(config)
  return bindConnector(db)
}

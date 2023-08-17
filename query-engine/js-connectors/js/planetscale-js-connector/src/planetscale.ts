import * as planetScale from '@planetscale/database'
import type { Config as PlanetScaleConfig } from '@planetscale/database'
import { bindConnector, bindTransaction, Debug } from '@jkomyno/prisma-js-connector-utils'
import type { Connector, ResultSet, Query, ConnectorConfig, Queryable, Transaction } from '@jkomyno/prisma-js-connector-utils'
import { type PlanetScaleColumnType, fieldToColumnType } from './conversion'
import { createDeferred, Deferred } from './deferred'

const debug = Debug('prisma:js-connector:planetscale')

export type PrismaPlanetScaleConfig = ConnectorConfig & Partial<PlanetScaleConfig>

class RollbackError extends Error {
  constructor() {
    super('ROLLBACK')
    this.name = 'RollbackError'

    if (Error.captureStackTrace) {
      Error.captureStackTrace(this, RollbackError);
    }
  }
}


class PlanetScaleQueryable<ClientT extends planetScale.Connection | planetScale.Transaction> implements Queryable {
  readonly flavour = 'mysql'
  constructor(protected client: ClientT) {
  }

    /**
   * Execute a query given as SQL, interpolating the given parameters.
   */
  async queryRaw(query: Query): Promise<ResultSet> {
    const tag = '[js::query_raw]'
    debug(`${tag} %O`, query)

    const { fields, insertId: lastInsertId, rows: results } = await this.performIO(query)

    const columns = fields.map(field => field.name)
    const resultSet: ResultSet = {
      columnNames: columns,
      columnTypes: fields.map(field => fieldToColumnType(field.type as PlanetScaleColumnType)),
      rows: results.map(result => columns.map(column => result[column])),
      lastInsertId,
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

    return await this.client.execute(sql, values)
  }
}

class PlanetScaleTransaction extends PlanetScaleQueryable<planetScale.Transaction> implements Transaction {
  constructor(tx: planetScale.Transaction, private txDeferred: Deferred<void>, private txResultPromise: Promise<void>) {
    super(tx)
  }

  commit(): Promise<void> {
    const tag = '[js::commit]'
    debug(`${tag} committing transaction`)
    this.txDeferred.resolve()
    return this.txResultPromise;
  }

  rollback(): Promise<void> {
    const tag = '[js::rollback]'
    debug(`${tag} rolling back the transaction`)
    this.txDeferred.reject(new RollbackError())
    return this.txResultPromise;
  }

}

class PrismaPlanetScale extends PlanetScaleQueryable<planetScale.Connection> implements Connector {
  constructor(config: PrismaPlanetScaleConfig) {
    const client = planetScale.connect(config)

    super(client)
    
  }

  async startTransaction(isolationLevel?: string) {
    return new Promise<Transaction>((resolve) => {
      const txResultPromise = this.client.transaction(async tx => {
        if (isolationLevel) {
          await tx.execute(`SET TRANSACTION ISOLATION LEVEL ${isolationLevel}`)
        }
        const [txDeferred, deferredPromise] = createDeferred<void>()
        const txWrapper = new PlanetScaleTransaction(tx, txDeferred, txResultPromise)

        resolve(bindTransaction(txWrapper));

        return deferredPromise
      }).catch(error => {
        // Rollback error is ignored (so that tx.rollback() won't crash)
        // any other error is legit and is re-thrown
        if (!(error instanceof RollbackError)) {
          return Promise.reject(error)
        }
        
        return undefined
      });
    })
  }
}

export const createPlanetScaleConnector = (config: PrismaPlanetScaleConfig): Connector => {
  const db = new PrismaPlanetScale(config)
  return bindConnector(db)
}

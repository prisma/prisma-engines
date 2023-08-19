import * as pg from 'pg'
import { bindConnector, bindTransaction, Debug } from '@jkomyno/prisma-js-connector-utils'
import type { Connector, ResultSet, Query, ConnectorConfig, Queryable, Transaction } from '@jkomyno/prisma-js-connector-utils'
import { fieldToColumnType } from './conversion'

const debug = Debug('prisma:js-connector:pg')

export type PrismaPgConfig = ConnectorConfig

type StdClient = pg.Pool
type TransactionClient = pg.PoolClient

class PgQueryable<ClientT extends StdClient | TransactionClient> implements Queryable {
    readonly flavour = 'postgres'
    constructor(protected readonly client: ClientT) {
    }

    /**
     * Execute a query given as SQL, interpolating the given parameters.
     */
    async queryRaw(query: Query): Promise<ResultSet> {
        const tag = '[js::query_raw]'
        debug(`${tag} %O`, query)

        const { fields, rows: results } = await this.performIO(query)

        const columns = fields.map(field => field.name)
        const resultSet: ResultSet = {
            columnNames: columns,
            columnTypes: fields.map(field => fieldToColumnType(field.dataTypeID)),
            rows: results.map(result => columns.map(column => result[column])),
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

        const { rowCount } = await this.performIO(query)
        return rowCount
    }

    /**
     * Run a query against the database, returning the result set.
     * Should the query fail due to a connection error, the connection is
     * marked as unhealthy.
     */
    private async performIO(query: Query) {
        const { sql, args: values } = query

        return await this.client.query(sql, values)
    }
}

class PgTransaction extends PgQueryable<TransactionClient> implements Transaction {
    constructor(client: pg.PoolClient) {
        super(client)
    }

    async commit(): Promise<void> {
        const tag = '[js::commit]'
        debug(`${tag} committing transaction`)
        try {
            await this.client.query('COMMIT');
        } finally {
            this.client.release()
        }

    }

    async rollback(): Promise<void> {
        const tag = '[js::rollback]'
        debug(`${tag} rolling back the transaction`)
        try {
            await this.client.query('ROLLBACK');
        } finally {
            this.client.release()
        }
    }
}

class PrismaPg extends PgQueryable<StdClient> implements Connector {

    constructor(config: PrismaPgConfig) {
        const { url: connectionString } = config;
        const client = new pg.Pool({
            connectionString,
            ssl: {
                rejectUnauthorized: false,
            },
        })
        super(client)
    }

    async startTransaction(isolationLevel?: string): Promise<Transaction> {
        const connection = await this.client.connect()
        await connection.query('BEGIN')
        if (isolationLevel) {
            await connection.query(`SET TRANSACTION ISOLATION LEVEL ${isolationLevel}`)

        }

        return bindTransaction(new PgTransaction(connection))
    }

    async close() { }
}

export const createPgConnector = (config: PrismaPgConfig): Connector => {
    const db = new PrismaPg(config)
    return bindConnector(db)
}

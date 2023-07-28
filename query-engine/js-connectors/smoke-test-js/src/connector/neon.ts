import { Pool, PoolConfig, neonConfig } from '@neondatabase/serverless'
import type { Closeable, Connector, ResultSet, Query } from '../engines/types/Library.js'
import { ColumnType } from '../engines/types/Library.js'

import ws from 'ws';
neonConfig.webSocketConstructor = ws;

/**
 * This is a simplification of quaint's value inference logic. Take a look at quaint's conversion.rs
 * module to see how other attributes of the field packet such as the field length are used to infer
 * the correct quaint::Value variant.
 */
function fieldToColumnType(fieldTypeId: number): ColumnType {
  switch (fieldTypeId) {
    case 16: // BOOL
    case 21: // INT2
    case 23: // INT4
      return ColumnType.Int32
    case 20: // INT8
    case 1700: // numeric
      return ColumnType.Int64
    case 700: // FLOAT4
      return ColumnType.Float
    case 701: // FLOAT8
      return ColumnType.Double
    case 25: // TEXT
    case 1043: // VARCHAR
      return ColumnType.Text
    case 1042: // BPCHAR
      return ColumnType.Char
    case 1082: // DATE
      return ColumnType.Date
    case 1083: // TIME
      return ColumnType.Time
    case 1114: // TIMESTAMP
      return ColumnType.DateTime
    case 3802: // JSONB
      return ColumnType.Json
    default:
      if (fieldTypeId >= 10000) {
        // Postgres Custom Types
        return ColumnType.Enum
      }
      throw new Error(`Unsupported column type: ${fieldTypeId}`)
  }
}

type NeonConfig = PoolConfig;

class PrismaNeon implements Connector, Closeable {
  private pool: Pool
  private versionPromise: Promise<string>
  private isRunning: boolean = true
  flavor = "postgres"

  constructor(config: NeonConfig) {
    this.pool = new Pool(config)
    // lazily retrieve the version and store it into `maybeVersion`
    this.versionPromise = new Promise((resolve, reject) => {
      this.pool.query('SELECT VERSION()')
        .then((results) => resolve(results.rows[0]['version']))
        .catch((error) => reject(error));
    });
  }

  async close(): Promise<void> {
    if (this.isRunning) {
      await this.pool.end()
      this.isRunning = false
    }
  }

  /**
   * Returns false, if connection is considered to not be in a working state.
   */
  async isHealthy(): Promise<boolean> {
    try {
      return await this.versionPromise !== undefined && this.isRunning
    } catch {
      return false
    }
  }

  /**
   * Execute a query given as SQL, interpolating the given parameters.
   */
  async queryRaw(query: Query): Promise<ResultSet> {
    const { sql, args: values } = query
    console.log(sql, values)
    const { fields, rows: results } = await this.pool.query(sql, values)
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
    const { sql, args: values } = query
    const { rowCount } = await this.pool.query(sql, values)
    return rowCount
  }

  /**
   * Return the version of the underlying database, queried directly from the
   * source. This corresponds to the `version()` function on PostgreSQL for
   * example. The version string is returned directly without any form of
   * parsing or normalization.
   */
  version(): Promise<string | undefined> {
    return this.versionPromise
  }
}

export const createNeonConnector = (config: NeonConfig): Connector & Closeable => {
  const db = new PrismaNeon(config)
  return db
}

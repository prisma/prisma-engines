import { NeonQueryFunction, Pool, PoolConfig, neon, neonConfig, types } from '@neondatabase/serverless'
import type { Closeable, Connector, ResultSet, Query } from '../engines/types/Library.js'
import { ColumnType } from '../engines/types/Library.js'

/**
 * This is a simplification of quaint's value inference logic. Take a look at quaint's conversion.rs
 * module to see how other attributes of the field packet such as the field length are used to infer
 * the correct quaint::Value variant.
 */
function fieldToColumnType(fieldTypeId: number): ColumnType {
  switch (fieldTypeId) {
    case 16: // BOOL
      return ColumnType.Boolean
    case 21: // INT2
    case 23: // INT4
      return ColumnType.Int32
    case 20: // INT8
      return ColumnType.Int64
    case 1700: // Numeric
      return ColumnType.Numeric
    case 700: // FLOAT4
      return ColumnType.Float
    case 701: // FLOAT8
      return ColumnType.Double
    case 25: // TEXT
    case 1043: // VARCHAR
      return ColumnType.Text
    case 1042: // BPCHAR
      return ColumnType.Text
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

// return string instead of JavaScript Date object
types.setTypeParser(1082, date => date);
types.setTypeParser(1083, date => date);
types.setTypeParser(1114, date => date);

export let lastQuery: any;
export let lastResult: any;

class PrismaNeon implements Connector, Closeable {
  private pool?: Pool
  private sql?: NeonQueryFunction<false, true>
  private maybeVersion?: string
  private isRunning: boolean = true
  flavor = "postgres"
  isHttp: boolean

  constructor(config: NeonConnectorConfig) {
    if (config.httpMode) {
      this.isHttp = true
      if (!config.connectionString) {
        throw Error('connectionString is required for http mode')
      }
      this.sql = neon(config.connectionString, { fullResults: true })
      // lazily retrieve the version and store it into `maybeVersion`
      this.sql('SELECT VERSION()').then((results) => {
        this.maybeVersion = results.rows[0]['version']
      })
    } else {
      this.isHttp = false
      this.pool = new Pool(config)
      // lazily retrieve the version and store it into `maybeVersion`
      this.pool.query('SELECT VERSION()').then((results) => {
        this.maybeVersion = results.rows[0]['version']
      })
    }
  }

  async close(): Promise<void> {
    if (this.isRunning) {
      if (!this.isHttp) {
        await this.pool!.end()
      }
      this.isRunning = false
    }
  }

  /**
   * Returns false, if connection is considered to not be in a working state.
   */
  isHealthy(): boolean {
    const result = this.maybeVersion !== undefined
      && this.isRunning
    return result
  }

  /**
   * Execute a query given as SQL, interpolating the given parameters.
   */
  async queryRaw(query: Query): Promise<ResultSet> {
    lastQuery = query;
    const { sql, args: values } = query
    let fields;
    let results;
    if (this.isHttp) {
      const { fields: fields_, rows: rows_ } = await this.sql!(sql, values)
      results = rows_
      fields = fields_
    } else {
      const { fields: fields_, rows: rows_ } = await this.pool!.query(sql, values)
      results = rows_
      fields = fields_
    }
    const columns = fields.map(field => field.name)
    const columnTypes = fields.map(field => fieldToColumnType(field.dataTypeID))
    const resultSet: ResultSet = {
      columnNames: columns,
      columnTypes,
      rows: results.map(result => columns.map((column, i) => {
        if (columnTypes[i] == ColumnType.Boolean) {
          return result[column] ? 1 : 0
        } else {
          return result[column]
        }
      })),
    }
    lastResult = resultSet;
    return resultSet
  }

  /**
   * Execute a query given as SQL, interpolating the given parameters and
   * returning the number of affected rows.
   * Note: Queryable expects a u64, but napi.rs only supports u32.
   */
  async executeRaw(query: Query): Promise<number> {
    const { sql, args: values } = query
    if (this.isHttp) {
      const { rowCount } = await this.sql!(sql, values)
      return rowCount
    } else {
      const { rowCount } = await this.pool!.query(sql, values)
      return rowCount
    }
  }

  /**
   * Return the version of the underlying database, queried directly from the
   * source. This corresponds to the `version()` function on PostgreSQL for
   * example. The version string is returned directly without any form of
   * parsing or normalization.
   */
  version(): Promise<string | undefined> {
    return Promise.resolve(this.maybeVersion)
  }
}

type NeonConnectorConfig = NeonConfig & { httpMode?: boolean }

export const createNeonConnector = (config: NeonConnectorConfig): Connector & Closeable => {
  const db = new PrismaNeon(config)
  return db
}

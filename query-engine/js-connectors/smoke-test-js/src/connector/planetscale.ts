import * as planetScale from '@planetscale/database'
import type { Closeable, Connector, ResultSet, Query } from '../engines/types/Library.js'
import { ColumnType } from '../engines/types/Library.js'

// See: https://github.com/planetscale/vitess-types/blob/06235e372d2050b4c0fff49972df8111e696c564/src/vitess/query/v16/query.proto#L108-L218
type PlanetScaleColumnType
  = 'NULL_TYPE' // unsupported
  | 'INT8'
  | 'UINT8'
  | 'INT16'
  | 'UINT16'
  | 'INT24'
  | 'UINT24'
  | 'INT32'
  | 'UINT32'
  | 'INT64'
  | 'UINT64'
  | 'FLOAT32'
  | 'FLOAT64'
  | 'TIMESTAMP'
  | 'DATE'
  | 'TIME'
  | 'DATETIME'
  | 'YEAR'
  | 'DECIMAL'
  | 'TEXT'
  | 'BLOB'
  | 'VARCHAR'
  | 'VARBINARY'
  | 'CHAR'
  | 'BINARY'
  | 'BIT'
  | 'ENUM'
  | 'SET' // unsupported
  | 'TUPLE' // unsupported
  | 'GEOMETRY'
  | 'JSON'
  | 'EXPRESSION' // unsupported
  | 'HEXNUM'
  | 'HEXVAL'
  | 'BITNUM'

/**
 * This is a simplification of quaint's value inference logic. Take a look at quaint's conversion.rs
 * module to see how other attributes of the field packet such as the field length are used to infer
 * the correct quaint::Value variant.
 */
function fieldToColumnType(field: PlanetScaleColumnType): ColumnType {
  switch (field) {
    case 'INT8':
    case 'UINT8':
    case 'INT16':
    case 'UINT16':
    case 'INT24':
    case 'UINT24':
    case 'INT32':
    case 'UINT32':
    case 'YEAR':
      return ColumnType.Int32
    case 'INT64':
    case 'UINT64':
      return ColumnType.Int64
    case 'FLOAT32':
      return ColumnType.Float
    case 'FLOAT64':
      return ColumnType.Double
    case 'TIMESTAMP':
    case 'DATETIME':
      return ColumnType.DateTime
    case 'DATE':
      return ColumnType.Date
    case 'TIME':
      return ColumnType.Time
    case 'DECIMAL':
      return ColumnType.Numeric
    case 'CHAR':
      return ColumnType.Char
    case 'TEXT':
    case 'VARCHAR':
      return ColumnType.Text
    case 'ENUM':
      return ColumnType.Enum
    case 'JSON':
      return ColumnType.Json
    case 'BLOB':
    case 'BINARY':
    case 'VARBINARY':
    case 'BIT':
    case 'BITNUM':
    case 'HEXNUM':
    case 'HEXVAL':
    case 'GEOMETRY':
      return ColumnType.Bytes
    default:
      throw new Error(`Unsupported column type: ${field}`)
  }
}

type PlanetScaleConfig =
  & {
    fetch?: planetScale.Config['fetch'],
  }
  & (
    {
      host: string,
      username: string,
      password: string,
    } | {
      url: string,
    }
  )

class PrismaPlanetScale implements Connector, Closeable {
  readonly flavor = 'mysql'

  private isRunning: boolean = true
  private _isHealthy: boolean = true
  private _version: string | undefined = undefined
  private client: planetScale.Connection

  constructor(config: PlanetScaleConfig) {
    this.client = planetScale.connect(config)
  }

  async close(): Promise<void> {
    if (this.isRunning) {
      this.isRunning = false
    }
  }

  /**
   * Returns false, if connection is considered to not be in a working state.
   */
  isHealthy(): boolean {
    return this.isRunning && this._isHealthy
  }

  /**
   * Execute a query given as SQL, interpolating the given parameters.
   */
  async queryRaw(query: Query): Promise<ResultSet> {
    const { fields, rows: results } = await this.performIO(query)

    const columns = fields.map(field => field.name)
    const resultSet: ResultSet = {
      columnNames: columns,
      columnTypes: fields.map(field => fieldToColumnType(field.type as PlanetScaleColumnType)),
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
    const { rowsAffected } = await this.performIO(query)
    return rowsAffected
  }

  /**
   * Return the version of the underlying database, queried directly from the
   * source. This corresponds to the `version()` function on PostgreSQL for
   * example. The version string is returned directly without any form of
   * parsing or normalization.
   */
  async version(): Promise<string | undefined> {
    if (this._version) {
      return Promise.resolve(this._version)
    }

    const { rows } = await this.performIO({ sql: 'SELECT @@version', args: [] })
    const version = rows[0]['@@version'] as string
    return version
  }

  /**
   * Run a query against the database, returning the result set.
   * Should the query fail due to a connection error, the connection is
   * marked as unhealthy.
   */
  private async performIO(query: Query) {
    const { sql, args: values } = query

    try {
      return await this.client.execute(sql, values, { as: 'object' })
    } catch (e) {
      const error = e as Error & { code: string }
      
      if (['ENOTFOUND', 'EAI_AGAIN'].includes(error.code)) {
        this._isHealthy = false
      }

      throw e
    }
  }
}

export const createPlanetScaleConnector = (config: PlanetScaleConfig): Connector & Closeable => {
  const db = new PrismaPlanetScale(config)
  return db
}

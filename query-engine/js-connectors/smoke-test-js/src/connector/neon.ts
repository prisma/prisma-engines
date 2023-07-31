import { Pool, neonConfig, types } from '@neondatabase/serverless'
import type { NeonConfig } from '@neondatabase/serverless'
import ws from 'ws'
import type { Closeable, Connector, ResultSet, Query } from '../engines/types/Library.js'
import { ColumnType } from '../engines/types/Library.js'
import { ConnectorConfig } from './util.js'

neonConfig.webSocketConstructor = ws

const NeonColumnType = types.builtins

/**
 * This is a simplification of quaint's value inference logic. Take a look at quaint's conversion.rs
 * module to see how other attributes of the field packet such as the field length are used to infer
 * the correct quaint::Value variant.
 */
function fieldToColumnType(fieldTypeId: number): ColumnType {
  switch (fieldTypeId) {
    case NeonColumnType['INT2']:
    case NeonColumnType['INT4']:
      return ColumnType.Int32
    case NeonColumnType['INT8']:
      return ColumnType.Int64
    case NeonColumnType['FLOAT4']:
      return ColumnType.Float
    case NeonColumnType['FLOAT8']:
      return ColumnType.Double
    case NeonColumnType['BOOL']:
      return ColumnType.Boolean
    case NeonColumnType['DATE']:
      return ColumnType.Date
    case NeonColumnType['TIME']:
      return ColumnType.Time
    case NeonColumnType['TIMESTAMP']:
      return ColumnType.DateTime
    case NeonColumnType['NUMERIC']:
      return ColumnType.Numeric
    case NeonColumnType['BPCHAR']:
      return ColumnType.Char
    case NeonColumnType['TEXT']:
    case NeonColumnType['VARCHAR']:
      return ColumnType.Text
    case NeonColumnType['JSONB']:
      return ColumnType.Json
    default:
      if (fieldTypeId >= 10000) {
        // Postgres Custom Types
        return ColumnType.Enum
      }
      throw new Error(`Unsupported column type: ${fieldTypeId}`)
  }
}

// return string instead of JavaScript Date object
types.setTypeParser(NeonColumnType.DATE, date => date);
types.setTypeParser(NeonColumnType.TIME, date => date);
types.setTypeParser(NeonColumnType.TIMESTAMP, date => date);

export type PrismaNeonConfig = ConnectorConfig & Partial<Omit<NeonConfig, 'connectionString'>>

class PrismaNeon implements Connector, Closeable {
  readonly flavor = 'postgres'
  
  private pool: Pool
  private isRunning: boolean = true
  private _isHealthy: boolean = true
  private _version: string | undefined = undefined
  
  constructor(config: PrismaNeonConfig) {
    const { url: connectionString, ...rest } = config
    this.pool = new Pool({ connectionString, ...rest })
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
    const { rowCount: rowsAffected } = await this.performIO(query)
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

    const { rows } = await this.performIO({ sql: 'SELECT VERSION()', args: [] })
    this._version = rows[0]['version'] as string
    return this._version
  }

  /**
   * Run a query against the database, returning the result set.
   * Should the query fail due to a connection error, the connection is
   * marked as unhealthy.
   */
  private async performIO(query: Query) {
    const { sql, args: values } = query

    try {
      return await this.pool.query(sql, values)
    } catch (e) {
      const error = e as Error & { code: string }
      
      if (['ENOTFOUND', 'EAI_AGAIN'].includes(error.code)) {
        this._isHealthy = false
      }

      throw e
    }
  }
}

export const createNeonConnector = (config: PrismaNeonConfig): Connector & Closeable => {
  const db = new PrismaNeon(config)
  return db
}

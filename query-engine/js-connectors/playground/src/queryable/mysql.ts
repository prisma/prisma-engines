import * as mysql from 'mysql2/promise'

import { Closeable, Queryable, ResultSet, Query, ColumnType } from '../engines/types/Library'

// See: https://github.com/mysql/mysql-server/blob/ea7087d885006918ad54458e7aad215b1650312c/include/field_types.h#L52-L87
enum MySQLColumnType {
  Decimal,
  Tiny,
  Short,
  Long,
  Float,
  Double,
  Null,
  Timestamp,
  LongLong,
  Int24,
  Date,
  Time,
  Datetime,
  Year,
  Newdate, /**< Internal to MySQL. Not used in protocol */
  Varchar,
  Bit,
  Timestamp2,
  Datetime2,   /**< Internal to MySQL. Not used in protocol */
  Time2,       /**< Internal to MySQL. Not used in protocol */
  TypedArray, /**< Used for replication only */
  Invalid = 243,
  Bool = 244, /**< Currently just a placeholder */
  Json = 245,
  Newdecimal = 246,
  Enum = 247,
  Set = 248,
  TinyBlob = 249,
  MediumBlob = 250,
  LongBlob = 251,
  Blob = 252,
  VarString = 253,
  String = 254,
  Geometry = 255
}

/**
 * This is a simplification of quaint's value inference logic. Take a look at quaint's conversion.rs
 * module to see how other attributes of the field packet such as the field length are used to infer
 * the correct quaint::Value variant.
 */
function fieldToColumnType(field: mysql.FieldPacket): ColumnType {
  const columnTypeMapping: Readonly<Record<MySQLColumnType, ColumnType | undefined>> = {
    [MySQLColumnType.Decimal]: undefined,
    [MySQLColumnType.Tiny]: undefined,
    [MySQLColumnType.Short]: undefined,
    [MySQLColumnType.Long]: ColumnType.Int64,
    [MySQLColumnType.Float]: undefined,
    [MySQLColumnType.Double]: undefined,
    [MySQLColumnType.Null]: undefined,
    [MySQLColumnType.Timestamp]: undefined,
    [MySQLColumnType.LongLong]: undefined,
    [MySQLColumnType.Int24]: undefined,
    [MySQLColumnType.Date]: undefined,
    [MySQLColumnType.Time]: undefined,
    [MySQLColumnType.Datetime]: undefined,
    [MySQLColumnType.Year]: undefined,
    [MySQLColumnType.Newdate]: undefined,
    [MySQLColumnType.Varchar]: undefined,
    [MySQLColumnType.Bit]: undefined,
    [MySQLColumnType.Timestamp2]: undefined,
    [MySQLColumnType.Datetime2]: undefined,
    [MySQLColumnType.Time2]: undefined,
    [MySQLColumnType.TypedArray]: undefined,
    [MySQLColumnType.Invalid]: undefined,
    [MySQLColumnType.Bool]: undefined,
    [MySQLColumnType.Json]: undefined,
    [MySQLColumnType.Newdecimal]: undefined,
    [MySQLColumnType.Enum]: undefined,
    [MySQLColumnType.Set]: undefined,
    [MySQLColumnType.TinyBlob]: undefined,
    [MySQLColumnType.MediumBlob]: undefined,
    [MySQLColumnType.LongBlob]: undefined,
    [MySQLColumnType.Blob]: undefined,
    [MySQLColumnType.VarString]: ColumnType.Text,
    [MySQLColumnType.String]: undefined,
    [MySQLColumnType.Geometry]: undefined
  };

  let colType = columnTypeMapping[field.type]
  if (colType === undefined) {
    console.log(`Unsupported mysql type: ${field.type}`)
  }
  return colType
}

class PrismaMySQL implements Queryable, Closeable {
  private pool: mysql.Pool
  private maybeVersion?: string
  private isRunning: boolean = true

  constructor(connectionString: string) {
    console.log(`[nodejs] initializing mysql connection pool: ${connectionString}`)
    this.pool = mysql.createPool(connectionString)

    // lazily retrieve the version and store it into `maybeVersion`
    this.pool.query<mysql.RowDataPacket[]>({
      sql: 'SELECT @@version, @@GLOBAL.version',
    }).then(([results, _]) => {
      this.maybeVersion = results[0]['@@version']
    })
  }

  async close(): Promise<void> {
    console.log('[nodejs] calling close() on connection pool')
    if (this.isRunning) {
      this.isRunning = false
      await this.pool.end()
      console.log('[nodejs] closed connection pool')
    }
  }

  /**
   * Returns false, if connection is considered to not be in a working state.
   */
  isHealthy(): boolean {
    const result = this.maybeVersion !== undefined
      && this.isRunning
    console.log(`[nodejs] isHealthy: ${result}`)
    return result
  }

  /**
   * Execute a query given as SQL, interpolating the given parameters.
   */
  async queryRaw(params: Query): Promise<ResultSet> {
    console.log('[nodejs] calling queryRaw', params)
    const [results, fields] = await this.pool.query<mysql.RowDataPacket[]>({
      sql: params.sql,
      values: params.args,
      rowsAsArray: false,
    })
    console.log('[nodejs] after query')

    const columns = fields.map(field => field.name)
    const resultSet: ResultSet = {
      columnNames: columns,
      // TODO: cancel the promise and fail in case the column type is not supported by the driver
      columnTypes: fields.map(field => fieldToColumnType(field)),
      rows: results.map(result => columns.map(column => result[column]))
    };
    console.log('[nodejs] resultSet', resultSet)

    return resultSet
  }

  /**
   * Execute a query given as SQL, interpolating the given parameters and
   * returning the number of affected rows.
   * Note: Queryable expects a u64, but napi.rs only supports u32.
   */
  async executeRaw(params: Query): Promise<number> {
    console.log('[nodejs] calling executeRaw', params)
    const [{ affectedRows }, _] = await this.pool.execute<mysql.ResultSetHeader>({
      sql: params.sql,
      values: params.args,
    })
    return affectedRows
  }

  /**
   * Return the version of the underlying database, queried directly from the
   * source. This corresponds to the `version()` function on PostgreSQL for
   * example. The version string is returned directly without any form of
   * parsing or normalization.
   */
  version(): string | undefined {
    return this.maybeVersion
  }
}

export const createMySQLQueryable = (connectionString: string): Queryable & Closeable => {
  const db = new PrismaMySQL(connectionString)
  return db
}

import { ColumnTypeEnum } from './const'

export type ColumnType = typeof ColumnTypeEnum[keyof typeof ColumnTypeEnum]

export interface ResultSet {
  /**
   * List of column types appearing in a database query, in the same order as `columnNames`.
   * They are used within the Query Engine to convert values from JS to Quaint values.
   */
  columnTypes: Array<ColumnType>

  /**
   * List of column names appearing in a database query, in the same order as `columnTypes`.
   */
  columnNames: Array<string>

  /**
   * List of rows retrieved from a database query.
   * Each row is a list of values, whose length matches `columnNames` and `columnTypes`.
   */
  rows: Array<Array<any>>

  /**
   * The last ID of an `INSERT` statement, if any.
   * This is required for `AUTO_INCREMENT` columns in MySQL and SQLite-flavoured databases.
   */
  lastInsertId?: string
}

export interface Query {
  sql: string
  args: Array<any>
}

export type Connector = {
  readonly flavour: 'mysql' | 'postgres',

  queryRaw: (params: Query) => Promise<ResultSet>
  executeRaw: (params: Query) => Promise<number>
  version: () => Promise<string | undefined>
  isHealthy: () => boolean
}

export type Closeable = {
  close: () => Promise<void>
}

export type ConnectorConfig
  = {
    host: string,
    username: string,
    password: string,
    url: never
  } | {
    url: string,
  }

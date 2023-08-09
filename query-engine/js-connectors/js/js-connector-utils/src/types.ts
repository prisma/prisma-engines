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
  rows: Array<Array<unknown>>

  /**
   * The last ID of an `INSERT` statement, if any.
   * This is required for `AUTO_INCREMENT` columns in MySQL and SQLite-flavoured databases.
   */
  lastInsertId?: string
}

export type Query = {
  sql: string
  args: Array<unknown>
}

export type Connector = {
  readonly flavour: 'mysql' | 'postgres',

  /**
   * Execute a query given as SQL, interpolating the given parameters,
   * and returning the type-aware result set of the query.
   * 
   * This is the preferred way of executing `SELECT` queries.
   */
  queryRaw: (params: Query) => Promise<ResultSet>

  /**
   * Execute a query given as SQL, interpolating the given parameters,
   * and returning the number of affected rows.
   * 
   * This is the preferred way of executing `INSERT`, `UPDATE`, `DELETE` queries,
   * as well as transactional queries.
   */
  executeRaw: (params: Query) => Promise<number>

  /**
   * Return the version of the underlying database, queried directly from the
   * source.
   */
  version: () => Promise<string | undefined>

  /**
   * Returns true, if connection is considered to be in a working state.
   */
  isHealthy: () => boolean

  /**
   * Closes the connection to the database, if any.
   */
  close: () => Promise<void>
}

/**
 * Base configuration for a connector.
 */
export type ConnectorConfig = {
  /**
   * The connection string of the database server to connect to.
   */
  url: string,
}

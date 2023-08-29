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

export interface Queryable  {
  readonly flavour: 'mysql' | 'postgres'

  /**
   * Execute a query given as SQL, interpolating the given parameters,
   * and returning the type-aware result set of the query.
   * 
   * This is the preferred way of executing `SELECT` queries.
   */
  queryRaw(params: Query): Promise<ResultSet>

  /**
   * Execute a query given as SQL, interpolating the given parameters,
   * and returning the number of affected rows.
   * 
   * This is the preferred way of executing `INSERT`, `UPDATE`, `DELETE` queries,
   * as well as transactional queries.
   */
  executeRaw(params: Query): Promise<number>
}

export interface Connector extends Queryable {
  /**
   * Starts new transation with the specified isolation level
   * @param isolationLevel 
   */
  startTransaction(isolationLevel?: string): Promise<Transaction>

  /**
   * Opens a connection to the database.
   */
  connect: () => Promise<void>

  /**
   * Closes the connection to the database, if any.
   */
  disconnect: () => Promise<void>
}

export interface Transaction extends Queryable {
  /**
   * Commit the transaction
   */
  commit(): Promise<void>
  /**
   * Rolls back the transaction.
   */
  rollback(): Promise<void>
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

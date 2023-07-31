import { setTimeout } from 'node:timers/promises'

import { Closeable, ColumnType, Query, Connector, ResultSet } from '../engines/types/Library.js'
import type { ConnectorConfig } from './util.js'

type MockSQLConfig = ConnectorConfig

class MockSQL implements Connector, Closeable {
  readonly flavor = 'mysql'
  
  private maybeVersion?: string
  private isRunning: boolean = true

  constructor(_config: MockSQLConfig) {
    // lazily retrieve the version and store it into `maybeVersion`
    setTimeout(50)
      .then(() => {
        this.maybeVersion = 'x.y.z'
      })
  }

  async close(): Promise<void> {
    console.log('[nodejs] calling close() on connection pool')
    if (this.isRunning) {
      this.isRunning = false
      await setTimeout(150)
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
    await setTimeout(100)

    const resultSet: ResultSet = {
      columnNames: ['id', 'firstname', 'company_id'],
      columnTypes: [ColumnType.Int64, ColumnType.Text, ColumnType.Int64],
      rows: [
        [1, 'Alberto', 1],
        [2, 'Tom', 1],
      ],
    }
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
    await setTimeout(100)

    const affectedRows = 32
    return affectedRows
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

export const createMockConnector = (config: MockSQLConfig): Connector & Closeable => {
  const db = new MockSQL(config)
  return db
}

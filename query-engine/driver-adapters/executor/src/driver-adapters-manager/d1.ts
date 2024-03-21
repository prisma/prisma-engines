import path from 'node:path'
import * as S from '@effect/schema/Schema'
import { PrismaD1 } from '@prisma/adapter-d1'
import { DriverAdapter } from '@prisma/driver-adapter-utils'
import { getPlatformProxy } from 'wrangler'
import type { D1Database, D1Result } from '@cloudflare/workers-types'

import { __dirname, runBatch } from '../utils'
import type { ConnectParams, DriverAdaptersManager } from './index'
import type { DriverAdapterTag, EnvForAdapter } from '../types'
import { D1Tables } from '../types/d1'

const TAG = 'd1' as const satisfies DriverAdapterTag
type TAG = typeof TAG

export class D1Manager implements DriverAdaptersManager {
  #driver: D1Database
  #dispose: () => Promise<void>
  #adapter?: DriverAdapter

  private constructor(private env: EnvForAdapter<TAG>, driver: D1Database, dispose: () => Promise<void>) {
    this.#driver = driver
    this.#dispose = dispose
  }

  static async setup(env: EnvForAdapter<TAG>, migrationScript?: string) {
    const { env: cfBindings, dispose } = await getPlatformProxy<{ D1_DATABASE: D1Database }>({
      configPath: path.join(__dirname, "../wrangler.toml"),
    })

    const { D1_DATABASE } = cfBindings

    /* prisma migrate reset */
    console.warn('[D1] Resetting database')
    await migrateReset(D1_DATABASE)

    /* prisma migrate diff */
    if (migrationScript) {
      console.warn('[D1] Running database migration script')
      await migrateDiff(D1_DATABASE, migrationScript)
    }

    return new D1Manager(env, D1_DATABASE, dispose)
  }

  async connect({}: ConnectParams) {
    this.#adapter = new PrismaD1(this.#driver)
    return this.#adapter
  }

  async teardown() {
    await this.#dispose()
  }
}

async function migrateDiff(D1_DATABASE: D1Database, migrationScript: string) {
  // Note: when running a script with multiple statements, D1 fails with
  // `D1_ERROR: A prepared SQL statement must contain only one statement.`
  // We thus need to run each statement separately, splitting the script by `;`.
  const sqlStatements = migrationScript.split(';')
  const preparedStatements = sqlStatements.map((sqlStatement) => D1_DATABASE.prepare(sqlStatement))
  await runBatch(D1_DATABASE, preparedStatements)
}

async function migrateReset(D1_DATABASE: D1Database) {
  let { results: rawTables } = ((await D1_DATABASE.prepare(`PRAGMA main.table_list;`).run()) as D1Result)
  let tables = S
    .decodeUnknownSync(D1Tables, { onExcessProperty: 'preserve' })(rawTables)
    .filter((item) => !['_cf_KV', 'sqlite_schema', 'sqlite_sequence'].includes(item.name))

  // This may sometimes fail with `D1_ERROR: no such table: sqlite_sequence`,
  // so it needs to be outside of the batch transaction.
  // From the docs (https://www.sqlite.org/autoinc.html):
  // "The sqlite_sequence table is created automatically, if it does not already exist,
  // whenever a normal table that contains an AUTOINCREMENT column is created".
  try {
    await D1_DATABASE.prepare(`DELETE FROM "sqlite_sequence";`).run()
  } catch (_) {
    // Ignore the error, as the table may not exist.
    console.warn('Failed to reset sqlite_sequence table, but continuing with the reset.')
  }

  const batch = [] as string[]

  // Allow violating foreign key constraints on the batch transaction.
  // The foreign key constraints are automatically re-enabled at the end of the transaction, regardless of it succeeding.
  batch.push(`PRAGMA defer_foreign_keys = ${1};`)

  for (const table of tables) {
    if (table.type === 'view') {
      batch.push(`DROP VIEW IF EXISTS "${table.name}";`)
    } else {
      batch.push(`DROP TABLE IF EXISTS "${table.name}";`)
    }
  }

  const statements = batch.map((sql) => D1_DATABASE.prepare(sql))
  const batchResult = await runBatch(D1_DATABASE, statements)
  
  for (const { error } of batchResult) {
    if (error) {
      console.error('Error in batch: %O', error)
    }
  }
}

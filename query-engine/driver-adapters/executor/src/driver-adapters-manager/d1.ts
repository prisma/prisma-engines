import path from 'node:path'
import * as S from '@effect/schema/Schema'
import { PrismaD1 } from '@prisma/adapter-d1'
import { DriverAdapter } from '@prisma/driver-adapter-utils'
import { getPlatformProxy } from 'wrangler'
import type { D1Database, D1Response, D1Result } from '@cloudflare/workers-types'

import { __dirname, runBatch } from '../utils'
import type { ConnectParams, DriverAdaptersManager } from './index'
import type { DriverAdapterTag, EnvForAdapter } from '../types'
import { D1Indexes, D1Tables } from '../types/d1'

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
    .filter((item) => !['_cf_KV', 'sqlite_schema'].includes(item.name))

  const batch = [] as string[]

  // temporarily allow violating foreign key constraints
  batch.push(`PRAGMA defer_foreign_keys = ${1};`)

  for (const table of tables) {
    if (table.name === 'sqlite_sequence') {
      batch.push('DELETE FROM `sqlite_sequence`;')
    } else if (table.type === 'view') {
      batch.push(`DROP VIEW IF EXISTS "${table.name}";`)
    } else {
      // TODO: Consider stop polling indexes and test on CI, they're probabably automatically
      // deleted when their table is dropped.

      const { results: rawIndexes } = ((await D1_DATABASE.prepare(`PRAGMA main.index_list("${table.name}");`).run()) as D1Result)

      const indexes = S
        .decodeUnknownSync(D1Indexes, { onExcessProperty: 'preserve' })(rawIndexes)

      const indexesToDrop = indexes
        .filter((index) => !['c'].includes(index.origin))
        .map((index) => `DROP INDEX IF EXISTS "${index.name}";`)

      batch.push(`DROP TABLE IF EXISTS "${table.name}";`)
      batch.push(...indexesToDrop)
    }
  }

  // stop violating foreign key constraints
  batch.push(`PRAGMA defer_foreign_keys = ${0};`)

  const statements = batch.map((sql) => D1_DATABASE.prepare(sql))
  const batchResult = (await runBatch(D1_DATABASE, statements)) as D1Response[]

  for (const { error } of batchResult) {
    if (error) {
      console.error('Error in batch: %O', error)
    }
  }
}

import path from 'node:path'
import * as S from '@effect/schema/Schema'
import { PrismaD1 } from '@prisma/adapter-d1'
import type {
  SqlDriverAdapter,
  SqlDriverAdapterFactory,
} from '@prisma/driver-adapter-utils'
import { getPlatformProxy } from 'wrangler'
import type { D1Database, D1Result } from '@cloudflare/workers-types'

import { __dirname, runBatch } from '../utils.js'
import type {
  DriverAdaptersManager,
  SetupDriverAdaptersInput,
} from './index.js'
import type { DriverAdapterTag, EnvForAdapter } from '../types/index.js'
import { D1Tables } from '../types/d1.js'

const TAG = 'd1' as const satisfies DriverAdapterTag
type TAG = typeof TAG

export class D1Manager implements DriverAdaptersManager {
  #dispose: () => Promise<void>
  #factory: SqlDriverAdapterFactory
  #adapter?: SqlDriverAdapter

  private constructor(
    private env: EnvForAdapter<TAG>,
    driver: D1Database,
    dispose: () => Promise<void>,
  ) {
    this.#factory = new PrismaD1(driver)
    this.#dispose = dispose
  }

  static async setup(
    env: EnvForAdapter<TAG>,
    { migrationScript }: SetupDriverAdaptersInput,
  ) {
    const { env: cfBindings, dispose } = await getPlatformProxy<{
      D1_DATABASE: D1Database
    }>({
      configPath: path.join(__dirname, '../wrangler.toml'),
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

  factory() {
    return this.#factory
  }

  async connect() {
    return (this.#adapter ??= await this.#factory.connect())
  }

  async teardown() {
    await this.#dispose()
  }

  connector(): 'sqlite' {
    return 'sqlite'
  }
}

async function migrateDiff(D1_DATABASE: D1Database, migrationScript: string) {
  // Note: when running a script with multiple statements, D1 fails with
  // `D1_ERROR: A prepared SQL statement must contain only one statement.`
  // We thus need to run each statement separately, splitting the script by `;`.
  const sqlStatements = migrationScript.split(';')
  const preparedStatements = sqlStatements.map((sqlStatement) =>
    D1_DATABASE.prepare(sqlStatement),
  )
  await runBatch(D1_DATABASE, preparedStatements)
}

// Rows returned by `PRAGMA foreign_key_list`; `table` is the referenced parent.
const D1ForeignKeys = S.array(S.struct({ table: S.string }))

async function migrateReset(D1_DATABASE: D1Database) {
  let { results: rawTables } = (await D1_DATABASE.prepare(
    `PRAGMA main.table_list;`,
  ).run()) as D1Result
  let tables = S.decodeUnknownSync(D1Tables, { onExcessProperty: 'preserve' })(
    rawTables,
  ).filter(
    (item) =>
      !(
        ['sqlite_schema', 'sqlite_sequence'].includes(item.name) ||
        // excludes `_cf_KV`, `_cf_METADATA`, etc.
        // Related to https://github.com/drizzle-team/drizzle-orm/issues/3728#issuecomment-2740994190.
        /^(_cf_[A-Z]+).*$/.test(item.name)
      ),
  )

  // This may sometimes fail with `D1_ERROR: no such table: sqlite_sequence`,
  // so it needs to be outside of the batch transaction.
  // From the docs (https://www.sqlite.org/autoinc.html):
  // "The sqlite_sequence table is created automatically, if it does not already exist,
  // whenever a normal table that contains an AUTOINCREMENT column is created".
  try {
    await D1_DATABASE.prepare(`DELETE FROM "sqlite_sequence";`).run()
  } catch (e) {
    // Ignore the error, as the table may not exist.
    console.warn(
      'Failed to reset sqlite_sequence table, but continuing with the reset.',
    )
  }

  // Map each table to the tables its foreign keys reference (its parents).
  // Self-references don't constrain the drop order.
  const parentsOf = new Map<string, Set<string>>()
  for (const table of tables) {
    if (table.type !== 'table') {
      continue
    }
    const { results: rawForeignKeys } = (await D1_DATABASE.prepare(
      `PRAGMA foreign_key_list("${table.name}");`,
    ).run()) as D1Result
    const foreignKeys = S.decodeUnknownSync(D1ForeignKeys, {
      onExcessProperty: 'preserve',
    })(rawForeignKeys)
    parentsOf.set(
      table.name,
      new Set(
        foreignKeys.map((fk) => fk.table).filter((name) => name !== table.name),
      ),
    )
  }

  // Drop a table only after every table referencing it is gone: in a batch, a
  // child's DROP no longer compiles once its FK parent is dropped
  // (`no such table: main.<parent>`, wrangler >= 4.126), and
  // `defer_foreign_keys` doesn't help because it defers enforcement, not name
  // resolution. Views are never FK parents, so they drop right away. Tables in
  // a reference cycle can't be ordered and keep their original order.
  const remaining = new Set(tables.map((table) => table.name))
  const ordered = [] as typeof tables
  let progress = true
  while (remaining.size > 0 && progress) {
    progress = false
    for (const table of tables) {
      if (!remaining.has(table.name)) {
        continue
      }
      const isReferenced = tables.some(
        (other) =>
          other.name !== table.name &&
          remaining.has(other.name) &&
          parentsOf.get(other.name)?.has(table.name),
      )
      if (!isReferenced) {
        ordered.push(table)
        remaining.delete(table.name)
        progress = true
      }
    }
  }
  for (const table of tables) {
    if (remaining.has(table.name)) {
      ordered.push(table)
    }
  }

  const batch = [] as string[]

  // Allow violating foreign key constraints on the batch transaction.
  // The foreign key constraints are automatically re-enabled at the end of the transaction, regardless of it succeeding.
  batch.push(`PRAGMA defer_foreign_keys = ${1};`)

  for (const table of ordered) {
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

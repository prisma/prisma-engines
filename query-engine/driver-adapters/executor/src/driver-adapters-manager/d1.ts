import path from 'node:path'
import { PrismaD1 } from '@prisma/adapter-d1'
import { DriverAdapter } from '@prisma/driver-adapter-utils'
import { getPlatformProxy } from 'wrangler'
import type { D1Database } from '@cloudflare/workers-types'

import { __dirname } from '../utils'
import type { ConnectParams, DriverAdaptersManager } from './index'
import type { DriverAdapterTag, EnvForAdapter } from '../types'

const TAG = 'd1' as const satisfies DriverAdapterTag
type TAG = typeof TAG

export class D1Manager implements DriverAdaptersManager {
  #driver: D1Database
  #dispose: () => Promise<void>
  #adapter?: DriverAdapter

  constructor(private env: EnvForAdapter<TAG>, driver: D1Database, dispose: () => Promise<void>) {
    this.#driver = driver
    this.#dispose = dispose
  }

  static async setup(env: EnvForAdapter<TAG>, migrationScript?: string) {
    const { env: cfBindings, dispose } = await getPlatformProxy<{ D1_DATABASE: D1Database }>({
      configPath: path.join(__dirname, "../wrangler.toml"),
    })

    const { D1_DATABASE } = cfBindings

    if (migrationScript) {
      console.warn('Running migration script for D1 database')
      console.warn(migrationScript)

      // Note: when running a script with multiple statements, D1 fails with
      // `D1_ERROR: A prepared SQL statement must contain only one statement.`
      // We thus need to run each statement separately, splitting the script by `;`.
      const sqlStatements = migrationScript.split(';')
      const preparedStatements = sqlStatements.map((sqlStatement) => D1_DATABASE.prepare(sqlStatement))
      await D1_DATABASE.batch(preparedStatements)
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

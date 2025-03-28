import { PrismaPg } from '@prisma/adapter-pg'
import type { SqlMigrationAwareDriverAdapterFactory, SqlDriverAdapter } from '@prisma/driver-adapter-utils'
import { postgresSchemaName, postgresOptions } from '../utils'
import type { DriverAdaptersManager, SetupDriverAdaptersInput } from './index'
import type { DriverAdapterTag, EnvForAdapter } from '../types'

const TAG = 'pg' as const satisfies DriverAdapterTag
type TAG = typeof TAG

export class PgManager implements DriverAdaptersManager {
  #factory: SqlMigrationAwareDriverAdapterFactory
  #adapter?: SqlDriverAdapter

  private constructor(private env: EnvForAdapter<TAG>, { url }: SetupDriverAdaptersInput) {
    const schemaName = postgresSchemaName(url)
    this.#factory = new PrismaPg(postgresOptions(url), {
      schema: schemaName,
    })
  }

  static async setup(env: EnvForAdapter<TAG>, input: SetupDriverAdaptersInput) {
    return new PgManager(env, input)
  }

  factory() {
    return this.#factory
  }

  async connect() {
    this.#adapter = await this.#factory.connect()
    return this.#adapter
  }

  async teardown() {
    await this.#adapter?.dispose()
  }
}

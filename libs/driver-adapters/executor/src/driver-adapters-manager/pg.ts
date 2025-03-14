import { PrismaPg } from '@prisma/adapter-pg'
import { SqlDriverAdapter } from '@prisma/driver-adapter-utils'
import { postgresSchemaName, postgresOptions } from '../utils'
import type { ConnectParams, DriverAdaptersManager } from './index'
import type { DriverAdapterTag, EnvForAdapter } from '../types'

const TAG = 'pg' as const satisfies DriverAdapterTag
type TAG = typeof TAG

export class PgManager implements DriverAdaptersManager {
  #adapter?: SqlDriverAdapter

  private constructor(private env: EnvForAdapter<TAG>) {}

  static async setup(env: EnvForAdapter<TAG>) {
    return new PgManager(env)
  }

  async connect({ url }: ConnectParams) {
    const schemaName = postgresSchemaName(url)
    const factory = new PrismaPg(postgresOptions(url), {
      schema: schemaName,
    })

    this.#adapter = await factory.connect()
    return this.#adapter
  }

  async teardown() {
    await this.#adapter?.dispose()
  }
}

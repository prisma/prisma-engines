import { PrismaPg } from '@prisma/adapter-pg'
import { pg } from '@prisma/bundled-js-drivers'
import { DriverAdapter } from '@prisma/driver-adapter-utils'
import { postgresSchemaName, postgresOptions } from '../utils'
import type { ConnectParams, DriverAdaptersManager } from './index'
import type { DriverAdapterTag, EnvForAdapter } from '../types'

const TAG = 'pg' as const satisfies DriverAdapterTag
type TAG = typeof TAG

export class PgManager implements DriverAdaptersManager {
  #driver?: pg.Pool
  #adapter?: DriverAdapter

  private constructor(private env: EnvForAdapter<TAG>) {}

  static async setup(env: EnvForAdapter<TAG>) {
    return new PgManager(env)
  }

  async connect({ url }: ConnectParams) {
    const schemaName = postgresSchemaName(url)

    this.#driver = new pg.Pool(postgresOptions(url))
    this.#adapter = new PrismaPg(this.#driver, { schema: schemaName }) as DriverAdapter

    return this.#adapter
  }

  async teardown() {
    await this.#driver?.end()
  }
}

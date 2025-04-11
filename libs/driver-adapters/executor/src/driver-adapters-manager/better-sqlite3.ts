import { PrismaBetterSQLite3 } from '@prisma/adapter-better-sqlite3'
import type {
  SqlDriverAdapter,
  SqlMigrationAwareDriverAdapterFactory,
} from '@prisma/driver-adapter-utils'
import type { DriverAdaptersManager, SetupDriverAdaptersInput } from './index'
import type { DriverAdapterTag, EnvForAdapter } from '../types'

const TAG = 'better-sqlite3' as const satisfies DriverAdapterTag
type TAG = typeof TAG

export class BetterSQLite3Manager implements DriverAdaptersManager {
  #factory: SqlMigrationAwareDriverAdapterFactory
  #adapter?: SqlDriverAdapter

  private constructor(
    private env: EnvForAdapter<TAG>,
    { url }: SetupDriverAdaptersInput
  ) {
    this.#factory = new PrismaBetterSQLite3({
      url,
    })
  }

  static async setup(env: EnvForAdapter<TAG>, input: SetupDriverAdaptersInput) {
    return new BetterSQLite3Manager(env, input)
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

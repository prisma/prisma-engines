import { PrismaMySQL2 } from '@prisma/adapter-mysql2'
import type {
  SqlDriverAdapter,
  SqlMigrationAwareDriverAdapterFactory,
} from '@prisma/driver-adapter-utils'
import type { DriverAdaptersManager, SetupDriverAdaptersInput } from './index'
import type { DriverAdapterTag, EnvForAdapter } from '../types'

const TAG = 'mysql2' as const satisfies DriverAdapterTag
type TAG = typeof TAG

export class MySQL2Manager implements DriverAdaptersManager {
  #factory: SqlMigrationAwareDriverAdapterFactory
  #adapter?: SqlDriverAdapter

  private constructor(
    private env: EnvForAdapter<TAG>,
    { url }: SetupDriverAdaptersInput,
  ) {
    const database = new URL(url).pathname.split('/').pop()
    this.#factory = new PrismaMySQL2({
      uri: url,
      database,
    })
  }

  static async setup(env: EnvForAdapter<TAG>, input: SetupDriverAdaptersInput) {
    return new MySQL2Manager(env, input)
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

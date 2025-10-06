import { PrismaLibSQL } from '@prisma/adapter-libsql'
import type {
  SqlDriverAdapter,
  SqlMigrationAwareDriverAdapterFactory,
} from '@prisma/driver-adapter-utils'
import type {
  DriverAdaptersManager,
  SetupDriverAdaptersInput,
} from './index.js'
import type { DriverAdapterTag, EnvForAdapter } from '../types/index.js'

const TAG = 'libsql' as const satisfies DriverAdapterTag
type TAG = typeof TAG

export class LibSQLManager implements DriverAdaptersManager {
  #factory: SqlMigrationAwareDriverAdapterFactory
  #adapter?: SqlDriverAdapter

  private constructor(
    private env: EnvForAdapter<TAG>,
    { url }: SetupDriverAdaptersInput,
  ) {
    this.#factory = new PrismaLibSQL({
      url,
      intMode: 'bigint',
    })
  }

  static async setup(env: EnvForAdapter<TAG>, input: SetupDriverAdaptersInput) {
    return new LibSQLManager(env, input)
  }

  factory() {
    return this.#factory
  }

  async connect() {
    return (this.#adapter ??= await this.#factory.connect())
  }

  async teardown() {
    await this.#adapter?.dispose()
  }

  connector(): 'sqlite' {
    return 'sqlite'
  }
}

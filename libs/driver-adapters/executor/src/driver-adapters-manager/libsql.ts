import { PrismaLibSQL } from '@prisma/adapter-libsql'
import { SqlDriverAdapter } from '@prisma/driver-adapter-utils'
import type { ConnectParams, DriverAdaptersManager } from './index'
import type { DriverAdapterTag, EnvForAdapter } from '../types'

const TAG = 'libsql' as const satisfies DriverAdapterTag
type TAG = typeof TAG

export class LibSQLManager implements DriverAdaptersManager {
  #adapter?: SqlDriverAdapter

  private constructor(private env: EnvForAdapter<TAG>) {}

  static async setup(env: EnvForAdapter<TAG>) {
    return new LibSQLManager(env)
  }

  async connect({ url }: ConnectParams) {
    const factory = new PrismaLibSQL({
      url,
      intMode: 'bigint',
    })

    this.#adapter = await factory.connect()
    return this.#adapter
  }

  async teardown() {
    await this.#adapter?.dispose()
  }
}

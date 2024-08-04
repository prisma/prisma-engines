import { PrismaLibSQL } from '@prisma/adapter-libsql'
import { libSql } from '@prisma/bundled-js-drivers'
import { DriverAdapter } from '@prisma/driver-adapter-utils'
import type { ConnectParams, DriverAdaptersManager } from './index'
import type { DriverAdapterTag, EnvForAdapter } from '../types'

const TAG = 'libsql' as const satisfies DriverAdapterTag
type TAG = typeof TAG

export class LibSQLManager implements DriverAdaptersManager {
  #driver?: libSql.Client
  #adapter?: DriverAdapter

  private constructor(private env: EnvForAdapter<TAG>) {}

  static async setup(env: EnvForAdapter<TAG>) {
    return new LibSQLManager(env)
  }

  async connect({ url }: ConnectParams) {
    this.#driver = libSql.createClient({ url, intMode: 'bigint' })
    this.#adapter = new PrismaLibSQL(this.#driver) as DriverAdapter

    return this.#adapter
  }

  async teardown() {}
}

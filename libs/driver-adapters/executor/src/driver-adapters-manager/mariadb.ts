import { PrismaMariaDb } from '@prisma/adapter-mariadb'
import type {
  SqlDriverAdapter,
  SqlDriverAdapterFactory,
} from '@prisma/driver-adapter-utils'
import type {
  DriverAdaptersManager,
  SetupDriverAdaptersInput,
} from './index.js'
import type { DriverAdapterTag, EnvForAdapter } from '../types/index.js'

const TAG = 'mariadb' as const satisfies DriverAdapterTag
type TAG = typeof TAG

export class MariaDbManager implements DriverAdaptersManager {
  #factory: SqlDriverAdapterFactory
  #adapter?: SqlDriverAdapter

  private constructor(
    private env: EnvForAdapter<TAG>,
    { url }: SetupDriverAdaptersInput,
  ) {
    this.#factory = new PrismaMariaDb(mariadbOptions(url))
  }

  static async setup(env: EnvForAdapter<TAG>, input: SetupDriverAdaptersInput) {
    return new MariaDbManager(env, input)
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

function mariadbOptions(urlStr: string) {
  const url = new URL(urlStr)
  const { username: user, password, hostname: host, port } = url
  const database = url.pathname && url.pathname.slice(1)

  return {
    user,
    password,
    database,
    host,
    port: Number(port),
    connectionLimit: 4,
  }
}

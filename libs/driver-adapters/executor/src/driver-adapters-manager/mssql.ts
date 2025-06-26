import { PrismaMssql } from '@prisma/adapter-mssql'
import type {
  SqlDriverAdapter,
  SqlDriverAdapterFactory,
} from '@prisma/driver-adapter-utils'
import type {
  DriverAdaptersManager,
  SetupDriverAdaptersInput,
} from './index.js'
import type { DriverAdapterTag, EnvForAdapter } from '../types/index.js'

const TAG = 'mssql' as const satisfies DriverAdapterTag
type TAG = typeof TAG

export class MssqlManager implements DriverAdaptersManager {
  #factory: SqlDriverAdapterFactory
  #adapter?: SqlDriverAdapter

  private constructor(
    private env: EnvForAdapter<TAG>,
    { url }: SetupDriverAdaptersInput,
  ) {
    const config = mssqlOptions(url)
    this.#factory = new PrismaMssql(config, { schema: config.schema })
  }

  static async setup(env: EnvForAdapter<TAG>, input: SetupDriverAdaptersInput) {
    return new MssqlManager(env, input)
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

  connector(): 'sqlserver' {
    return 'sqlserver'
  }
}

function mssqlOptions(url: string) {
  const [, server, port, database, schema, user, password] =
    url.match(
      /^sqlserver:\/\/([^:;]+):(\d+);database=([^;]+);schema=([^;]+);user=([^;]+);password=([^;]+);/,
    ) || []

  return {
    user,
    password,
    database,
    schema,
    server,
    port: Number(port),
    options: {
      trustServerCertificate: true,
    },
  }
}

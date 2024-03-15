import path from 'node:path'
import { PrismaD1 } from '@prisma/adapter-d1'
import { DriverAdapter } from '@prisma/driver-adapter-utils'
import { getPlatformProxy } from 'wrangler'
import type { D1Database } from '@cloudflare/workers-types'

import { __dirname } from '../utils'
import type { ConnectParams, DriverAdaptersManager } from './index'
import type { DriverAdapterTag, EnvForAdapter } from '../types'

const TAG = 'd1' as const satisfies DriverAdapterTag
type TAG = typeof TAG

export class D1Manager implements DriverAdaptersManager {
  #driver: D1Database
  #dispose: () => Promise<void>
  #adapter?: DriverAdapter

  constructor(private env: EnvForAdapter<TAG>, driver: D1Database, dispose: () => Promise<void>) {
    this.#driver = driver
    this.#dispose = dispose
  }

  static async setup(env: EnvForAdapter<TAG>) {
    const { env: cfBindings, dispose } = await getPlatformProxy<{ D1_DATABASE: D1Database }>({
      configPath: path.join(__dirname, "../wrangler.toml"),
    })
    
    return new D1Manager(env, cfBindings.D1_DATABASE, dispose)
  }

  async connect({ url }: ConnectParams) {
    return new PrismaD1(this.#driver)
  }

  async teardown() {
    await this.#dispose()
  }
}

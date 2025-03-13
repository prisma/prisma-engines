import { PrismaNeon } from '@prisma/adapter-neon'
import { neon } from '@prisma/bundled-js-drivers'
import { SqlDriverAdapter } from '@prisma/driver-adapter-utils'
import { WebSocket } from 'ws'
import { postgresSchemaName, postgresOptions } from '../utils'
import type { DriverAdaptersManager } from './index'
import type { DriverAdapterTag, EnvForAdapter } from '../types'

const TAG = 'neon:ws' as const satisfies DriverAdapterTag
type TAG = typeof TAG

type ConnectParams = {
  url: string
}

export class NeonWsManager implements DriverAdaptersManager {
  #driver?: neon.Pool
  #adapter?: SqlDriverAdapter

  private constructor(private env: EnvForAdapter<TAG>) {}

  static async setup(env: EnvForAdapter<TAG>) {
    return new NeonWsManager(env)
  }

  async connect({ url }: ConnectParams) {
    const { proxy_url: proxyUrl } = this.env.DRIVER_ADAPTER_CONFIG
    const { neonConfig, Pool } = neon

    neonConfig.wsProxy = () => proxyUrl
    neonConfig.webSocketConstructor = WebSocket
    neonConfig.useSecureWebSocket = false
    neonConfig.pipelineConnect = false

    const schemaName = postgresSchemaName(url)

    this.#driver = new Pool(postgresOptions(url))
    this.#adapter = new PrismaNeon(this.#driver, {
      schema: schemaName,
    }) as SqlDriverAdapter

    return this.#adapter
  }

  async teardown() {
    await this.#driver?.end()
  }
}

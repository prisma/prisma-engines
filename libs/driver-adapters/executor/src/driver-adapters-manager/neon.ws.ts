import { PrismaNeon } from '@prisma/adapter-neon'
import { neon } from '@prisma/bundled-js-drivers'
import type { SqlDriverAdapter, SqlDriverAdapterFactory } from '@prisma/driver-adapter-utils'
import { WebSocket } from 'ws'
import { postgresSchemaName, postgresOptions } from '../utils'
import type { DriverAdaptersManager, SetupDriverAdaptersInput } from './index'
import type { DriverAdapterTag, EnvForAdapter } from '../types'

const TAG = 'neon:ws' as const satisfies DriverAdapterTag
type TAG = typeof TAG

export class NeonWsManager implements DriverAdaptersManager {
  #factory: SqlDriverAdapterFactory
  #adapter?: SqlDriverAdapter

  private constructor(private env: EnvForAdapter<TAG>, { url }: SetupDriverAdaptersInput) {
    const schemaName = postgresSchemaName(url)
    this.#factory = new PrismaNeon(postgresOptions(url), {
      schema: schemaName,
    })
  }

  static async setup(env: EnvForAdapter<TAG>, input: SetupDriverAdaptersInput) {
    return new NeonWsManager(env, input)
  }

  factory() {
    return this.#factory
  }

  async connect() {
    const { proxy_url: proxyUrl } = this.env.DRIVER_ADAPTER_CONFIG
    const { neonConfig } = neon

    neonConfig.wsProxy = () => proxyUrl
    neonConfig.webSocketConstructor = WebSocket
    neonConfig.useSecureWebSocket = false
    neonConfig.pipelineConnect = false

    this.#adapter = await this.#factory.connect()

    return this.#adapter
  }

  async teardown() {
    await this.#adapter?.dispose()
  }
}

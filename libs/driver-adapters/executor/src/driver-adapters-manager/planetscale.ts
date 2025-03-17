import { PrismaPlanetScale } from '@prisma/adapter-planetscale'
import { SqlDriverAdapter } from '@prisma/driver-adapter-utils'
import { fetch } from 'undici'
import { copyPathName } from '../utils'
import type { ConnectParams, DriverAdaptersManager } from './index'
import type { DriverAdapterTag, EnvForAdapter } from '../types'

const TAG = 'planetscale' as const satisfies DriverAdapterTag
type TAG = typeof TAG

export class PlanetScaleManager implements DriverAdaptersManager {
  #adapter?: SqlDriverAdapter

  private constructor(private env: EnvForAdapter<TAG>) {}

  static async setup(env: EnvForAdapter<TAG>) {
    return new PlanetScaleManager(env)
  }

  async connect({ url }: ConnectParams) {
    const { proxy_url: proxyUrl } = this.env.DRIVER_ADAPTER_CONFIG

    const factory = new PrismaPlanetScale({
      // preserving path name so proxy url would look like real DB url
      url: copyPathName({ fromURL: url, toURL: proxyUrl }),
      fetch,
    })

    this.#adapter = await factory.connect()
    return this.#adapter
  }

  async teardown() {}
}

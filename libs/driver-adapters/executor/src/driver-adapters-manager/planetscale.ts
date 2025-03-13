import { PrismaPlanetScale } from '@prisma/adapter-planetscale'
import { planetScale } from '@prisma/bundled-js-drivers'
import { SqlDriverAdapter } from '@prisma/driver-adapter-utils'
import { fetch } from 'undici'
import { copyPathName } from '../utils'
import type { ConnectParams, DriverAdaptersManager } from './index'
import type { DriverAdapterTag, EnvForAdapter } from '../types'

const TAG = 'planetscale' as const satisfies DriverAdapterTag
type TAG = typeof TAG

export class PlanetScaleManager implements DriverAdaptersManager {
  #driver?: planetScale.Client
  #adapter?: SqlDriverAdapter

  private constructor(private env: EnvForAdapter<TAG>) {}

  static async setup(env: EnvForAdapter<TAG>) {
    return new PlanetScaleManager(env)
  }

  async connect({ url }: ConnectParams) {
    const { proxy_url: proxyUrl } = this.env.DRIVER_ADAPTER_CONFIG

    this.#driver = new planetScale.Client({
      // preserving path name so proxy url would look like real DB url
      url: copyPathName({ fromURL: url, toURL: proxyUrl }),
      fetch,
    })

    this.#adapter = new PrismaPlanetScale(this.#driver) as SqlDriverAdapter

    return this.#adapter
  }

  async teardown() {}
}

import { PrismaPlanetScale } from '@prisma/adapter-planetscale'
import type {
  SqlDriverAdapter,
  SqlDriverAdapterFactory,
} from '@prisma/driver-adapter-utils'
import { copyPathName } from '../utils'
import type { DriverAdaptersManager, SetupDriverAdaptersInput } from './index'
import type { DriverAdapterTag, EnvForAdapter } from '../types'

const TAG = 'planetscale' as const satisfies DriverAdapterTag
type TAG = typeof TAG

export class PlanetScaleManager implements DriverAdaptersManager {
  #factory: SqlDriverAdapterFactory
  #adapter?: SqlDriverAdapter

  private constructor(
    private env: EnvForAdapter<TAG>,
    { url }: SetupDriverAdaptersInput,
  ) {
    const { proxy_url: proxyUrl } = this.env.DRIVER_ADAPTER_CONFIG

    this.#factory = new PrismaPlanetScale({
      // preserving path name so proxy url would look like real DB url
      url: copyPathName({ fromURL: url, toURL: proxyUrl }),
    })
  }

  static async setup(env: EnvForAdapter<TAG>, input: SetupDriverAdaptersInput) {
    return new PlanetScaleManager(env, input)
  }

  factory() {
    return this.#factory
  }

  async connect() {
    this.#adapter = await this.#factory.connect()
    return this.#adapter
  }

  async teardown() {}
}

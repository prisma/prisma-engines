import type { DriverAdapter } from '@prisma/driver-adapter-utils'

export type ConnectParams = {
  url: string
}

export interface DriverAdaptersManager {
  connect: (params: ConnectParams) => Promise<DriverAdapter>
  teardown: () => Promise<void>
}

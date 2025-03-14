import type { SqlDriverAdapter } from '@prisma/driver-adapter-utils'

export type ConnectParams = {
  url: string
}

export interface DriverAdaptersManager {
  connect: (params: ConnectParams) => Promise<SqlDriverAdapter>
  teardown: () => Promise<void>
}

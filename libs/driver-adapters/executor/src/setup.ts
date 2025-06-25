import { match } from 'ts-pattern'
import type {
  SetupDriverAdaptersInput,
  DriverAdaptersManager,
} from './driver-adapters-manager'
import type { Env } from './types/index.js'
import { PgManager } from './driver-adapters-manager/pg.js'
import { NeonWsManager } from './driver-adapters-manager/neon.ws.js'
import { LibSQLManager } from './driver-adapters-manager/libsql.js'
import { PlanetScaleManager } from './driver-adapters-manager/planetscale.js'
import { D1Manager } from './driver-adapters-manager/d1.js'
import { BetterSQLite3Manager } from './driver-adapters-manager/better-sqlite3.js'
import { MssqlManager } from './driver-adapters-manager/mssql.js'

export async function setupDriverAdaptersManager(
  env: Env,
  input: SetupDriverAdaptersInput,
): Promise<DriverAdaptersManager> {
  return match(env)
    .with(
      { DRIVER_ADAPTER: 'pg' },
      async (env) => await PgManager.setup(env, input),
    )
    .with(
      { DRIVER_ADAPTER: 'neon:ws' },
      async (env) => await NeonWsManager.setup(env, input),
    )
    .with(
      { DRIVER_ADAPTER: 'libsql' },
      async (env) => await LibSQLManager.setup(env, input),
    )
    .with(
      { DRIVER_ADAPTER: 'planetscale' },
      async (env) => await PlanetScaleManager.setup(env, input),
    )
    .with(
      { DRIVER_ADAPTER: 'd1' },
      async (env) => await D1Manager.setup(env, input),
    )
    .with(
      { DRIVER_ADAPTER: 'better-sqlite3' },
      async (env) => await BetterSQLite3Manager.setup(env, input),
    )
    .with(
      { DRIVER_ADAPTER: 'mssql' },
      async (env) => await MssqlManager.setup(env, input),
    )
    .exhaustive()
}

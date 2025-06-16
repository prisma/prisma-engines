import { match } from 'ts-pattern'
import type {
  SetupDriverAdaptersInput,
  DriverAdaptersManager,
} from './driver-adapters-manager'
import type { Env } from './types'
import { PgManager } from './driver-adapters-manager/pg'
import { NeonWsManager } from './driver-adapters-manager/neon.ws'
import { LibSQLManager } from './driver-adapters-manager/libsql'
import { PlanetScaleManager } from './driver-adapters-manager/planetscale'
import { D1Manager } from './driver-adapters-manager/d1'
import { BetterSQLite3Manager } from './driver-adapters-manager/better-sqlite3'
import { MssqlManager } from './driver-adapters-manager/mssql'

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

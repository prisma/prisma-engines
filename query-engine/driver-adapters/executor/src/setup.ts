import { match } from 'ts-pattern';
import { type DriverAdaptersManager } from './driver-adapters-manager';
import type { Env } from './types';
import { PgManager } from "./driver-adapters-manager/pg";
import { NeonWsManager } from "./driver-adapters-manager/neon.ws";
import { LibSQLManager } from "./driver-adapters-manager/libsql";
import { PlanetScaleManager } from "./driver-adapters-manager/planetscale";
import { D1Manager } from "./driver-adapters-manager/d1";

export async function setupDriverAdaptersManager(
  env: Env,
  migrationScript?: string
): Promise<DriverAdaptersManager> {
  return match(env)
    .with({ DRIVER_ADAPTER: "pg" }, async (env) => await PgManager.setup(env))
    .with(
      { DRIVER_ADAPTER: "neon:ws" },
      async (env) => await NeonWsManager.setup(env)
    )
    .with(
      { DRIVER_ADAPTER: "libsql" },
      async (env) => await LibSQLManager.setup(env)
    )
    .with(
      { DRIVER_ADAPTER: "planetscale" },
      async (env) => await PlanetScaleManager.setup(env)
    )
    .with(
      { DRIVER_ADAPTER: "d1" },
      async (env) => await D1Manager.setup(env, migrationScript)
    )
    .exhaustive();
}

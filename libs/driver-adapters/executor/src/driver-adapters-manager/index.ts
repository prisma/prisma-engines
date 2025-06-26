import type {
  SqlDriverAdapter,
  SqlDriverAdapterFactory,
} from '@prisma/driver-adapter-utils'
import { Env, EnvForAdapter } from '../types'

export type SetupDriverAdaptersInput = {
  /**
   * The URL to the database to connect to.
   */
  url: string

  /**
   * The `prisma migrate diff --script` output to apply migrations before connecting to the database.
   * This is a temporary workaround only used by Cloudflare D1, and will be replaced by a full Wasm
   * migration solution in the future.
   * See: https://linear.app/prisma-company/issue/ORM-707/tests-prisma-engines-plug-in-testd-sets-into-sql-migration-tests-and
   */
  migrationScript?: string
}

export interface DriverAdaptersManager {
  /**
   * Access the Driver Adapter factory
   */
  factory: () => SqlDriverAdapterFactory

  /**
   * Creates a queryable instance from the Driver Adapter factory,
   * attempting a connection to the database.
   */
  connect: () => Promise<SqlDriverAdapter>

  /**
   * Closes the connection to the database and cleans up any used resources.
   */
  teardown: () => Promise<void>

  /**
   * Returns the connector used by the Manager.
   */
  connector: () => Env['CONNECTOR']
}

import type { ErrorCapturingSqlDriverAdapterFactory } from '@prisma/driver-adapter-utils'

/**
 * Note: this currently only works with the `--experimental-wasm-modules` flag.
 * Still, it's the easiest to use the wasm Schema Engine in this sandbox,
 * with type-safe bindings out of the box.
 */
import { SchemaEngine } from '@prisma/schema-engine-wasm'
import { __dirname } from './utils'

export type QueryLogCallback = (log: string) => void

export async function initSchemaEngine(
  adapterFactory: ErrorCapturingSqlDriverAdapterFactory,
): Promise<SchemaEngine> {
  return await SchemaEngine.new(adapterFactory)
}

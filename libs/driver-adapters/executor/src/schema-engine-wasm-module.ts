import type { ErrorCapturingSqlDriverAdapterFactory } from '@prisma/driver-adapter-utils'

/**
 * Note: this currently only works with the `--experimental-wasm-modules` flag.
 * Still, it's the easiest to use the wasm Schema Engine in this sandbox,
 * with type-safe bindings out of the box.
 */
import { SchemaEngine, type ConstructorOptions } from '@prisma/schema-engine-wasm'
import { __dirname } from './utils'

export { type ConstructorOptions } from '@prisma/schema-engine-wasm'

export async function initSchemaEngine(
  options: ConstructorOptions,
  debug: (log: string) => void,
  adapterFactory: ErrorCapturingSqlDriverAdapterFactory,
): Promise<SchemaEngine> {
  return await SchemaEngine.new(options, debug, adapterFactory)
}

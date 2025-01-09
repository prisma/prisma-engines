import type { DriverAdapter } from '@prisma/driver-adapter-utils'
import { __dirname } from './utils'

export type SchemaEngineParams = {
  // TODO: support multiple datamodels
  datamodel: string
}

export interface SchemaEngine {
  new(params: SchemaEngineParams, adapter: DriverAdapter): SchemaEngine
  debugPanic(): Promise<void>
  version(): Promise<string | undefined>
  reset(): Promise<void>
}

export type QueryLogCallback = (log: string) => void

export async function initSchemaEngine(
  params: SchemaEngineParams,
  adapter: DriverAdapter,
): Promise<SchemaEngine> {
  const { getSchemaEngineForProvider: getEngineForProvider } = await import('./schema-engine-wasm')
  const WasmSchemaEngine = (await getEngineForProvider(adapter.provider)) as SchemaEngine
  return new WasmSchemaEngine(params, adapter)
}

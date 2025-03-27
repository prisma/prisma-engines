import type { ErrorCapturingSqlDriverAdapterFactory } from '@prisma/driver-adapter-utils'

export type QueryLogCallback = (log: string) => void

export async function initSchemaEngine(
  adapterFactory: ErrorCapturingSqlDriverAdapterFactory,
) {
  const { SchemaEngine } = await import('@prisma/schema-engine-wasm')

  return await SchemaEngine.new(adapterFactory)
}

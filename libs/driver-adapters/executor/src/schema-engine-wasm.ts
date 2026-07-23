import fs from 'node:fs/promises'
import path from 'node:path'
import { __dirname } from './utils.js'
import type { SqlQueryable } from '@prisma/driver-adapter-utils'

const relativePath = '../../../../schema-engine/schema-engine-wasm/pkg'

const initializedModules = new Set<SqlQueryable['provider']>()

export async function getSchemaEngineForProvider(
  provider: SqlQueryable['provider'],
) {
  const engine = await import(`${relativePath}/schema_engine_bg.js`)

  if (!initializedModules.has(provider)) {
    const bytes = await fs.readFile(
      path.resolve(__dirname, relativePath, 'schema_engine_bg.wasm'),
    )

    const module = new WebAssembly.Module(bytes)
    const instance = new WebAssembly.Instance(module, {
      './schema_engine_bg.js': engine,
    })

    const wbindgen_start = instance.exports.__wbindgen_start as () => void
    engine.__wbg_set_wasm(instance.exports)
    wbindgen_start()
    initializedModules.add(provider)
  }

  return engine.SchemaEngine
}

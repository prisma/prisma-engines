import fs from 'node:fs/promises'
import path from 'node:path'
import { __dirname, normaliseProvider } from './utils'
import type { Queryable } from '@prisma/driver-adapter-utils'

const relativePath = '../../../../query-engine/query-engine-wasm/pkg'

const initializedModules = new Set<Queryable['provider']>()

export async function getQueryEngineForProvider(provider: Queryable['provider']) {
  const normalisedProvider = normaliseProvider(provider)
  const engine = await import(`${relativePath}/${normalisedProvider}/query_engine_bg.js`)

  if (!initializedModules.has(provider)) {
    const bytes = await fs.readFile(
      path.resolve(
        __dirname,
        relativePath,
        normalisedProvider,
        'query_engine_bg.wasm',
      ),
    )

    const module = new WebAssembly.Module(bytes)
    const instance = new WebAssembly.Instance(module, {
      './query_engine_bg.js': engine,
    })
    engine.__wbg_set_wasm(instance.exports)
    initializedModules.add(provider)
  }

  return engine.QueryEngine
}

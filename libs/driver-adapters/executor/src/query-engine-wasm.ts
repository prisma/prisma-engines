import fs from 'node:fs/promises'
import path from 'node:path'
import { __dirname, connectorWasmFileName } from './utils.js'
import { Env } from './types/env.js'

const relativePath = '../../../../query-engine/query-engine-wasm/pkg'

const initializedModules = new Set<string>()

export async function getQueryEngineForConnector(connector: Env['CONNECTOR']) {
  const normalisedConnector = connectorWasmFileName(connector)
  const engine = await import(
    `${relativePath}/${normalisedConnector}/query_engine_bg.js`
  )

  if (!initializedModules.has(normalisedConnector)) {
    const bytes = await fs.readFile(
      path.resolve(
        __dirname,
        relativePath,
        normalisedConnector,
        'query_engine_bg.wasm',
      ),
    )

    const module = new WebAssembly.Module(bytes)
    const instance = new WebAssembly.Instance(module, {
      './query_engine_bg.js': engine,
    })
    const wbindgen_start = instance.exports.__wbindgen_start as () => void
    engine.__wbg_set_wasm(instance.exports)
    wbindgen_start()
    initializedModules.add(normalisedConnector)
  }

  return engine.QueryEngine
}

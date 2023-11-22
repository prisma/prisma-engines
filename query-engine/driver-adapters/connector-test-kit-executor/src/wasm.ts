import * as wasm from '../../../query-engine-wasm/pkg/query_engine_bg.js'
import fs from 'node:fs/promises'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const dirname = path.dirname(fileURLToPath(import.meta.url))

const bytes = await fs.readFile(path.resolve(dirname, '..', '..', '..', 'query-engine-wasm', 'pkg', 'query_engine_bg.wasm'))
const module = new WebAssembly.Module(bytes) 
const instance = new WebAssembly.Instance(module, { './query_engine_bg.js': wasm })
wasm.__wbg_set_wasm(instance.exports);
wasm.init()

export const WasmQueryEngine = wasm.QueryEngine
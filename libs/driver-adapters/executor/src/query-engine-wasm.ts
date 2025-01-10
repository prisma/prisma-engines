import * as wasmPostgres from '../../../../query-engine/query-engine-wasm/pkg/postgresql/query_engine_bg.js'
import * as wasmMysql from '../../../../query-engine/query-engine-wasm/pkg/mysql/query_engine_bg.js'
import * as wasmSqlite from '../../../../query-engine/query-engine-wasm/pkg/sqlite/query_engine_bg.js'
import fs from 'node:fs/promises'
import path from 'node:path'
import { __dirname } from './utils'

const wasm = {
  postgres: wasmPostgres,
  mysql: wasmMysql,
  sqlite: wasmSqlite,
}

type EngineName = keyof typeof wasm

const initializedModules = new Set<EngineName>()

export async function getQueryEngineForProvider(provider: EngineName) {
  const engine = wasm[provider]
  if (!initializedModules.has(provider)) {
    const subDir = provider === 'postgres' ? 'postgresql' : provider
    const bytes = await fs.readFile(
      path.resolve(
        __dirname,
        '..',
        '..',
        '..',
        '..',
        'query-engine',
        'query-engine-wasm',
        'pkg',
        subDir,
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

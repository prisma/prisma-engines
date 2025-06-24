import * as wasmPostgres from '../../../../query-compiler/query-compiler-wasm/pkg/postgresql/query_compiler_bg.js'
import * as wasmMysql from '../../../../query-compiler/query-compiler-wasm/pkg/mysql/query_compiler_bg.js'
import * as wasmSqlite from '../../../../query-compiler/query-compiler-wasm/pkg/sqlite/query_compiler_bg.js'
import * as wasmSqlServer from '../../../../query-compiler/query-compiler-wasm/pkg/sqlserver/query_compiler_bg.js'
import * as wasmCockroachDb from '../../../../query-compiler/query-compiler-wasm/pkg/cockroachdb/query_compiler_bg.js'
import fs from 'node:fs/promises'
import path from 'node:path'
import { __dirname, connectorWasmFileName } from './utils.js'
import { Env } from './types/env.js'

const wasm = {
  postgresql: wasmPostgres,
  mysql: wasmMysql,
  sqlite: wasmSqlite,
  sqlserver: wasmSqlServer,
  cockroachdb: wasmCockroachDb,
}

const initializedModules = new Set<keyof typeof wasm>()

export async function getQueryCompilerForConnector(
  connector: Env['CONNECTOR'],
) {
  const normalisedConnector: keyof typeof wasm =
    connectorWasmFileName(connector)
  const engine = wasm[normalisedConnector]
  if (!initializedModules.has(normalisedConnector)) {
    const bytes = await fs.readFile(
      path.resolve(
        __dirname,
        '..',
        '..',
        '..',
        '..',
        'query-compiler',
        'query-compiler-wasm',
        'pkg',
        normalisedConnector,
        'query_compiler_bg.wasm',
      ),
    )
    const module = new WebAssembly.Module(bytes)
    const instance = new WebAssembly.Instance(module, {
      './query_compiler_bg.js': engine,
    })
    const wbindgen_start = instance.exports.__wbindgen_start as () => void
    engine.__wbg_set_wasm(instance.exports)
    wbindgen_start()
    initializedModules.add(normalisedConnector)
  }

  return engine.QueryCompiler
}
